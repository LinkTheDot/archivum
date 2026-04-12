#![allow(unused)]

use crate::config::SpanixScrubberConfig;
use crate::message_scraper::past_user_names::*;
use crate::message_scraper::username::*;
use crate::response_models::avaiable_logs::{AvailableLogs, LogEntry};
use crate::response_models::user_messages::UserMessages;
use crate::shutdown_request::ShutdownSignal;
use crate::user_chat_months::UserChatMonths;
use entities::*;
use entity_extensions::prelude::*;
use irc::proto::{Message as IrcMessage, message::Tag as IrcTag};
use sea_orm::prelude::Expr;
use sea_orm::sea_query::Query;
use sea_orm::sqlx::types::chrono::{DateTime, Utc};
use sea_orm::*;
use std::collections::HashSet;
use twitch_chat_tracker::channel::third_party_emote_list;
use twitch_chat_tracker::channel::third_party_emote_list_storage::EmoteListStorage;
use twitch_chat_tracker::errors::AppError;
use twitch_chat_tracker::irc_chat::message_parser;
use twitch_chat_tracker::irc_chat::message_parser::MessageParser;

mod past_user_names;
mod username;

impl SpanixScrubberConfig {
  pub async fn scrape_spanix_messages(self) -> ! {
    let shutdown_signal = ShutdownSignal::listen_for_sigterm();

    let users = self.retrieve_unscrubbed_users().await.unwrap();
    let total_users = users.len();
    let third_party_emote_list = EmoteListStorage::new(
      std::slice::from_ref(&self.channel.login_name),
      self.database_connection,
    )
    .await
    .unwrap();

    for (iteration, user) in users.into_iter().enumerate() {
      if shutdown_signal.was_requested() {
        tracing::info!(
          "Shutdown requested, exiting after processing {iteration}/{total_users} users"
        );

        std::process::exit(0);
      }

      tracing::info!(
        "Processing messages for user {} - {} | {iteration}/{total_users}",
        user.id,
        user.login_name,
      );

      let scrubbed_user_messages =
        match mark_user_as_began_scrubbing(&user, &self.channel, self.database_connection).await {
          Ok(scrubbed_user) => scrubbed_user,
          Err(error) => {
            tracing::error!(
              "Failed to create a scrubbed_user_messages for {user:?}. Reason: `{error}`"
            );

            continue;
          }
        };

      let Some(mut available_message_logs) = self.get_available_logs(&user).await else {
        continue;
      };

      let user_messages_result = self
        .get_available_entries(&user, available_message_logs)
        .await;
      let user_messages = match user_messages_result {
        Ok(user_messages) => user_messages,
        Err(error) => {
          tracing::error!(
            "Failed to retrieve messages for user `{} - {}`. Reason: {error}",
            user.id,
            user.login_name,
          );

          continue;
        }
      };

      let process_user_messages_result = self
        .process_user_messages(&user, user_messages, &third_party_emote_list)
        .await;
      if let Err(error) = process_user_messages_result {
        tracing::error!("Failed to process messages for user {user:?}. Reason: `{error}`");
      }

      scrubbed_user_messages
        .update_completed_status(self.database_connection)
        .await;
    }

    std::process::exit(0);
  }

  /// Retrieves the list of users that have yet to be scrubbed.
  //
  // SELECT
  //   twitch_user.id,
  //   twitch_user.twitch_id,
  //   twitch_user.display_name,
  //   twitch_user.login_name
  // FROM twitch_user
  // LEFT JOIN scrubbed_user_messages
  //   ON twitch_user.id = scrubbed_user_messages.twitch_user_id
  //   AND scrubbed_user_messages.channel_id = $1
  // WHERE scrubbed_user_messages.twitch_user_id IS NULL
  async fn retrieve_unscrubbed_users(&self) -> Result<Vec<twitch_user::Model>, AppError> {
    let channel_id = self.channel.id;

    twitch_user::Entity::find()
      .join(
        JoinType::LeftJoin,
        twitch_user::Relation::ScrubbedUserMessages
          .def()
          .on_condition(move |_left, right| {
            Condition::all()
              .add(Expr::col((right, scrubbed_user_messages::Column::ChannelId)).eq(channel_id))
          }),
      )
      .filter(scrubbed_user_messages::Column::TwitchUserId.is_null())
      .all(self.database_connection)
      .await
      .map_err(Into::into)
  }

  /// Retrieves the available message logs for a user from Spanix.
  /// None is returned if the list could not be retrieved or there were no logs.
  ///
  /// Filters for any months a user has existing messages on.
  pub async fn get_available_logs(&self, user: &twitch_user::Model) -> Option<AvailableLogs> {
    let available_message_logs_result =
      AvailableLogs::get_available_logs_for_user(&user.login_name, &self.channel.login_name).await;
    let mut available_message_logs = match available_message_logs_result {
      Ok(Some(available_logs)) => available_logs,
      Ok(None) => {
        tracing::warn!("No available logs found for `{}`.", user.login_name);

        return None;
      }
      Err(error) => {
        tracing::error!(
          "Failed to get available message logs for user {}. Reason: {error}",
          user.login_name
        );

        return None;
      }
    };

    let existing_chat_history = self.user_chat_months.get(user);

    available_message_logs.logs.retain(|log_entry| {
      let Some(existing_chat_history) = existing_chat_history else {
        return true;
      };

      !existing_chat_history.contains(log_entry)
    });

    if available_message_logs.logs.is_empty() {
      tracing::warn!("No available logs found for `{}`.", user.login_name);

      return None;
    }

    Some(available_message_logs)
  }

  pub async fn get_available_entries(
    &self,
    user: &twitch_user::Model,
    available_log_entries: AvailableLogs,
  ) -> Result<Vec<String>, AppError> {
    let mut all_user_messages = vec![];

    for LogEntry { year, month } in &available_log_entries.logs {
      tracing::info!("Getting messages on {year}-{month} for {}", user.login_name);
      let raw_messages =
        UserMessages::get_messages(&self.channel.login_name, &user.login_name, year, month).await?;
      let raw_messages: Vec<String> = raw_messages
        .messages
        .into_iter()
        .map(|user_message| user_message.raw)
        .collect();

      all_user_messages.extend(raw_messages);
    }

    Ok(all_user_messages)
  }

  async fn process_user_messages(
    &self,
    user: &twitch_user::Model,
    user_messages: Vec<String>,
    third_party_emote_list: &EmoteListStorage,
  ) -> Result<(), AppError> {
    let mut past_names: PastUserNames = PastUserNames::default();
    let message_count = user_messages.len();
    let percentages: Vec<usize> = (1..=10)
      .map(|divisor| (message_count as f32 * (divisor as f32 / 10.0)).ceil() as usize)
      .collect();

    tracing::info!("Processing messages for user `{user:?}`");

    for (iteration, message) in user_messages.into_iter().enumerate() {
      if percentages.contains(&iteration) {
        tracing::info!(
          "{}% ({iteration} | {message_count}) finished processing {user:?}'s messages",
          iteration * 100 / message_count,
        );
      }
      let mut irc_message = IrcMessage::from(message.as_str());

      past_names.check_for_user_name_changes(&mut irc_message, user);

      let message_parser = match MessageParser::new(&irc_message, third_party_emote_list) {
        Ok(message_parser) => message_parser,
        Err(error) => {
          tracing::error!(
            "Failed to create message parser for message. Message: {message:?}. Reason: {error}"
          );

          continue;
        }
      };

      if let Some(message_parser) = message_parser
        && let Err(error) = message_parser.parse(self.database_connection).await
      {
        tracing::error!("Failed to parse a message for user `{user:?}`. Reason: {error}");
      }
    }

    // Get them all, sort them by timestamp, and build up through time to determine
    // at a given time what the *previous* name was and what the new one *is*
    todo!("go through past_names and insert the discovered past names.");
  }
}

/// Create a new `scrubbed_user_messages` with the `completed` field set to false.
async fn mark_user_as_began_scrubbing(
  user: &twitch_user::Model,
  channel: &twitch_user::Model,
  database_connection: &DatabaseConnection,
) -> Result<scrubbed_user_messages::Model, AppError> {
  scrubbed_user_messages::ActiveModel {
    twitch_user_id: Set(user.id),
    channel_id: Set(channel.id),
    ..Default::default()
  }
  .insert(database_connection)
  .await
  .map_err(Into::into)
}
