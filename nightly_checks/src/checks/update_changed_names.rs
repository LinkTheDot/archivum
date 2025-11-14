use crate::rate_limiter::RateLimiter;
use anyhow::anyhow;
use database_connection::get_database_connection;
use entities::{twitch_user, twitch_user_name_change};
use entity_extensions::{prelude::TwitchUserExtensions, twitch_user::ChannelIdentifier};
use human_time::ToHumanTimeString;
use sea_orm::*;
use std::collections::{HashMap, HashSet};

pub struct DatabaseNameUpdateConfig<'a> {
  rate_limiter: RateLimiter,
  database_connection: &'a DatabaseConnection,
  all_login_and_display_names: HashSet<String>,
  user_list_by_twitch_ids: HashMap<i32, twitch_user::Model>,
  chunk_limit: usize,
  total_batches: usize,
}

impl DatabaseNameUpdateConfig<'_> {
  pub async fn new(max_requests_per_minute: usize, chunk_limit: usize) -> anyhow::Result<Self> {
    let database_connection = get_database_connection().await;
    let all_users = twitch_user::Entity::find().all(database_connection).await?;
    let rate_limiter = RateLimiter::new(max_requests_per_minute);
    let total_batches = all_users.len() / chunk_limit;
    let all_login_and_display_names: HashSet<String> = all_users
      .iter()
      .flat_map(|user| [user.login_name.to_owned(), user.display_name.to_owned()])
      .collect();
    let user_list_by_twitch_ids: HashMap<i32, twitch_user::Model> = all_users
      .into_iter()
      .map(|user| (user.twitch_id, user))
      .collect();

    Ok(Self {
      rate_limiter,
      database_connection,
      all_login_and_display_names,
      user_list_by_twitch_ids,
      chunk_limit,
      total_batches,
    })
  }

  pub async fn run(mut self) {
    tracing::info!("Total batch count: {}", self.total_batches);

    let user_list: Vec<&twitch_user::Model> = self.user_list_by_twitch_ids.values().collect();

    for (batch_number, user_batch) in user_list.chunks(self.chunk_limit).enumerate() {
      tracing::info!(
        "Processing batch number {}. Current tokens: {}",
        batch_number,
        self.rate_limiter.tokens()
      );
      let channel_identifiers: Vec<ChannelIdentifier<String>> = user_batch
        .iter()
        .map(|user| ChannelIdentifier::TwitchID(user.twitch_id.to_string()))
        .collect();

      if let Some(refresh_wait_time) = self.rate_limiter.request_tokens(self.chunk_limit) {
        tracing::info!(
          "Rate limit reached. Waiting for next refresh in {}.",
          refresh_wait_time.to_human_time_string()
        );

        tokio::time::sleep(refresh_wait_time).await;

        self.rate_limiter.reset_tokens();
      }

      let channel_list_query_result =
        twitch_user::Model::query_helix_for_channels_from_list(channel_identifiers.as_slice())
          .await;
      let mut update_channel_list = match channel_list_query_result {
        Ok(channel_list) => channel_list,
        Err(error) => {
          tracing::error!(
            "Failed to process batch number {}/{}. Reason: {}",
            batch_number,
            self.total_batches,
            error
          );

          continue;
        }
      };

      self.remove_unchanged_names(&mut update_channel_list);

      for channel in update_channel_list {
        let result = self
          .update_channel_and_insert_name_change(channel, batch_number)
          .await;

        if let Err(error) = result {
          tracing::error!("Failed to update a channel's name change. Reason: `{error}`");
        }
      }
    }
  }

  /// Removes any names from the list that have not changed on Twitch compared to their current database entry.
  fn remove_unchanged_names(&self, batch: &mut Vec<twitch_user::ActiveModel>) {
    batch.retain(|channel| {
      let Some(login_name) = channel.login_name.try_as_ref() else {
        tracing::error!("Channel {:?} is missing a login name", channel.id);

        return false;
      };
      let Some(display_name) = channel.display_name.try_as_ref() else {
        tracing::error!("Channel {:?} is missing a display name", channel.id);

        return false;
      };

      !self
        .all_login_and_display_names
        .contains(login_name.as_str())
        || !self
          .all_login_and_display_names
          .contains(display_name.as_str())
    });
  }

  async fn update_channel_and_insert_name_change(
    &self,
    mut channel_name_change: twitch_user::ActiveModel,
    current_batch_number: usize,
  ) -> anyhow::Result<()> {
    let Some(channel_twitch_id) = channel_name_change.twitch_id.try_as_ref() else {
      return Err(anyhow!("Missing twitch id: {:?}", channel_name_change));
    };
    let Some(new_login_name) = channel_name_change.login_name.try_as_ref().cloned() else {
      return Err(anyhow!("Missing login name: {:?}", channel_name_change));
    };
    let Some(new_display_name) = channel_name_change.display_name.try_as_ref().cloned() else {
      return Err(anyhow!("Missing display name: {:?}", channel_name_change));
    };
    let Some(corresponding_channel) = self.user_list_by_twitch_ids.get(channel_twitch_id) else {
      return Err(anyhow!(
        "Missing channel from user list: {:?}",
        channel_name_change
      ));
    };

    channel_name_change.id = Set(corresponding_channel.id);
    channel_name_change.twitch_id = Unchanged(corresponding_channel.twitch_id);

    tracing::info!("Updating: {:?}", channel_name_change);

    match channel_name_change.update(self.database_connection).await {
      Ok(insert_result) => {
        tracing::info!(
          "An item from batch {}/{} has been inserted. Result: {:?}",
          current_batch_number,
          self.total_batches,
          insert_result
        );

        let name_change = twitch_user_name_change::ActiveModel {
          twitch_user_id: Set(corresponding_channel.id),
          previous_login_name: Set(Some(corresponding_channel.login_name.clone())),
          previous_display_name: Set(Some(corresponding_channel.display_name.clone())),
          new_login_name: Set(Some(new_login_name)),
          new_display_name: Set(Some(new_display_name)),
          ..Default::default()
        };

        if let Err(error) = name_change.insert(self.database_connection).await {
          Err(anyhow!(
            "Failed to create a name change object for channel id {:?}. Reason: {:?}",
            corresponding_channel.id,
            error
          ))
        } else {
          Ok(())
        }
      }
      Err(error) => Err(anyhow!(
        "Failed to insert batch {}/{}. Reason: {}",
        current_batch_number,
        self.total_batches,
        error
      )),
    }
  }
}
