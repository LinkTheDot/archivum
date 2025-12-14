#![allow(unused)]

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime, TimeZone, Utc};
use database_connection::get_database_connection;
use entities::*;
use entity_extensions::prelude::TwitchUserExtensions;
use sea_orm::*;
use std::path::Path;
use tokio::fs;

/// # WARNING
///
/// Sets all messages as subscribed.
/// Sets all messages as no stream.
/// Sets all messages' `emote_only` column to 0.
/// Does not track third party emote usage.
/// Misses any Twitch emotes used that are not already in the database.
pub async fn run(filename: &str, year: i32, month: u32, day: u32, streamer_id: i32) {
  let channel = entities::twitch_user::Entity::find_by_id(streamer_id)
    .one(get_database_connection().await)
    .await
    .unwrap()
    .unwrap();
  let date = chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap();

  store_messages(filename, channel, date).await;

  // std::process::exit(0);
}

/// Expects:
///
/// ```
/// [hh:mm:ss] login_name: text content here
/// ```
///
/// The time is expected to be logged as EST.
///
/// Takes the filepath and date of the file.
///
/// To get the date:
/// ```
/// let date = chrono::NaiveDate::from_ymd_opt(2025, 4, 18).unwrap();
/// ```
///
/// # WARNING
///
/// Sets all messages as subscribed.
/// Sets all messages as no stream.
/// Sets all messages' `emote_only` column to 0.
/// Does not track third party emote usage.
/// Misses any Twitch emotes used that are not already in the database.
async fn store_messages<P: AsRef<Path>>(
  chatterino_logs_file_path: P,
  channel: twitch_user::Model,
  date: NaiveDate,
) {
  panic!("Implement the new emote usage table.");
  let chatterino_file_contents = fs::read_to_string(chatterino_logs_file_path).await.unwrap();
  let database_connection = get_database_connection().await;

  for line in chatterino_file_contents.lines() {
    if !line.contains(':') {
      tracing::info!("Skipping line {:?}", line);
    }

    let mut contents = line.splitn(3, " ");
    let Some(timestamp) = contents.next() else {
      tracing::error!("Failed to get timestamp from a message: {:?}", line);
      continue;
    };
    let Some(user_login) = contents.next() else {
      tracing::error!("Failed to get name from a message: {:?}", line);
      continue;
    };
    let Some(message) = contents.next() else {
      tracing::error!("Failed to get message from a message: {:?}", line);
      continue;
    };

    let user_login = extract_name(user_login);
    let get_user_result =
      twitch_user::Model::get_or_set_by_name(user_login, database_connection).await;
    let user = match get_user_result {
      Ok(user) => user,
      Err(error) => {
        tracing::error!(
          "Failed to get a user. Name: {:?}. Error: {:?}",
          user_login,
          error
        );
        continue;
      }
    };
    let timestamp = extract_timestamp(timestamp, date);
    // let emotes = find_emotes(message, database_connection).await;

    let message_active_model = stream_message::ActiveModel {
      is_first_message: Set(0),
      timestamp: Set(timestamp),
      emote_only: Set(0),
      contents: Set(Some(message.to_owned())),
      twitch_user_id: Set(user.id),
      channel_id: Set(channel.id),
      stream_id: Set(None),
      is_subscriber: Set(1),
      ..Default::default()
    };

    if let Err(error) = message_active_model.insert(database_connection).await {
      tracing::error!("Failed to insert a message. Reason: {:?}", error);
    }
  }
}

/// [hh:mm:ss] in EST
fn extract_timestamp(timestamp: &str, date: NaiveDate) -> DateTime<Utc> {
  let timestamp_slice = &timestamp[1..(timestamp.len() - 1)];

  let time = NaiveTime::parse_from_str(timestamp_slice, "%H:%M:%S").unwrap();
  let naive_dt = date.and_time(time);
  let est_offset = FixedOffset::west_opt(5 * 3600).unwrap();
  let dt_est = est_offset.from_local_datetime(&naive_dt).single().unwrap();

  dt_est.with_timezone(&Utc)
}

/// Remove the trailing `:` at the end of each name.
fn extract_name(name: &str) -> &str {
  if let Some(stripped) = name.strip_suffix(':') {
    stripped
  } else {
    name
  }
}

/// Returns JSON formatted {emote_id: usage_count} for the message
async fn find_emotes(message: &str, database_connection: &DatabaseConnection) -> serde_json::Value {
  let word_list: Vec<&str> = message.split(' ').collect();
  let emotes = emote::Entity::find()
    .filter(emote::Column::Name.is_in(word_list))
    .all(database_connection)
    .await
    .unwrap();
  let emotes: std::collections::HashMap<i32, i32> = emotes
    .into_iter()
    .map(|emote| {
      let id = emote.id;
      let count = message.matches(&emote.name).count() as i32;

      (id, count)
    })
    .collect();

  serde_json::to_value(emotes).unwrap()
}
