use crate::message_scraper::username::{MaybeUserName, UserName};
use entities::*;
use irc::proto::{Message as IrcMessage, message::Tag as IrcTag};
use sea_orm::sqlx::types::chrono::{self, DateTime, TimeZone, Utc};
use std::collections::{HashMap, hash_map::Entry};
use twitch_chat_tracker::errors::AppError;

const LOGIN_NAME_TAG: &str = "login";
const DISPLAY_NAME_TAG: &str = "display-name";
const TIMESTAMP_TAG: &str = "tmi-sent-ts";

#[derive(Default)]
pub(super) struct PastUserNames {
  known_names: HashMap<UserName, DateTime<Utc>>,
}

impl PastUserNames {
  /// Checks if the given UserName exists.
  /// If it does, the stored origin date is checked and update if the given date is older than the known date.
  /// If it doesn't it's added to the list of known names with the given origin date.
  pub fn check_for_user_name_changes(
    &mut self,
    irc_message: &mut IrcMessage,
    user: &twitch_user::Model,
  ) {
    let (maybe_username, name_origin_date) =
      set_irc_message_name_tags_if_different(irc_message, user);

    if let Some(new_name) = UserName::from_maybe_username(maybe_username, user)
      && let Some(name_origin_date) = name_origin_date
    {
      self.add_name(new_name, name_origin_date);
    }
  }

  pub fn get_timestamp_for_name(&self, name: &UserName) -> Option<&DateTime<Utc>> {
    self.known_names.get(name)
  }

  /// Add a name to the list of known names, updating the origin date if there's already an entry and the new date is older.
  pub fn add_name(&mut self, name: UserName, name_origin_date: DateTime<Utc>) {
    match self.known_names.entry(name) {
      Entry::Occupied(mut entry) => {
        let past_origin_date = entry.get();

        if past_origin_date > &name_origin_date {
          entry.insert(name_origin_date);
        }
      }
      Entry::Vacant(vacant) => {
        vacant.insert(name_origin_date);
      }
    }
  }

  pub fn contains(&mut self, name: &UserName) -> bool {
    self.known_names.contains_key(name)
  }
}

/// Sets the login and display name tags if they're different from the given user.
///
/// Returns the old login and display names if they existed and the timestamp tied to the message.
fn set_irc_message_name_tags_if_different(
  irc_message: &mut IrcMessage,
  user: &twitch_user::Model,
) -> (MaybeUserName, Option<DateTime<Utc>>) {
  let Some(tags) = &mut irc_message.tags else {
    return (MaybeUserName::default(), None);
  };

  let mut maybe_login_name = None;
  let mut maybe_display_name = None;
  let mut name_origin_date = None;

  for IrcTag(tag_name, tag_value) in tags.iter_mut() {
    match tag_name.as_str() {
      LOGIN_NAME_TAG => {
        if let Some(message_login) = tag_value
          && message_login != &user.login_name
        {
          maybe_login_name = Some(message_login.to_string());
          *message_login = user.login_name.clone()
        }
      }
      DISPLAY_NAME_TAG => {
        if let Some(message_display_name) = tag_value
          && message_display_name != &user.display_name
        {
          maybe_display_name = Some(message_display_name.to_string());
          *message_display_name = user.display_name.clone();
        }
      }
      TIMESTAMP_TAG => {
        if let Some(message_timestamp) = tag_value {
          match parse_timestamp(message_timestamp.clone()) {
            Ok(timestamp) => name_origin_date = Some(timestamp),
            Err(error) => {
              tracing::error!("Failed to parse timestamp for message. Error: {error}.");

              continue;
            }
          }
        }
      }
      _ => (),
    }
  }

  let maybe_username = MaybeUserName {
    login_name: maybe_login_name,
    display_name: maybe_display_name,
  };

  (maybe_username, name_origin_date)
}

fn parse_timestamp(timestamp: String) -> Result<DateTime<Utc>, AppError> {
  let Ok(timestamp) = timestamp.trim().parse::<i64>() else {
    return Err(AppError::FailedToParseValue {
      value_name: "timestamp",
      location: "parse timestamp",
      value: timestamp,
    });
  };
  let Some(timestamp) = chrono::Utc.timestamp_millis_opt(timestamp).single() else {
    return Err(AppError::CouldNotCreateTimestampWithUnixTimestamp(
      timestamp,
    ));
  };

  Ok(timestamp)
}
