use crate::data_transfer_objects::stream_message::StreamMessageUser;
use entities::stream_message;
use sea_orm::prelude::DateTimeUtc;

#[derive(sea_orm::FromQueryResult)]
pub struct StreamMessageWithUser {
  id: i32,
  is_first_message: i8,
  timestamp: DateTimeUtc,
  emote_only: i8,
  contents: Option<String>,
  twitch_user_id: i32,
  channel_id: i32,
  stream_id: Option<i32>,
  is_subscriber: i8,
  origin_id: Option<String>,
  is_from_subscription_message: i8,
  user_twitch_id: Option<i32>,
  user_login_name: Option<String>,
  user_display_name: Option<String>,
}

impl StreamMessageWithUser {
  pub const USER_TWITCH_ID_COLUMN: &str = "user_twitch_id";
  pub const USER_LOGIN_NAME_COLUMN: &str = "user_login_name";
  pub const USER_DISPLAY_NAME_COLUMN: &str = "user_display_name";

  pub fn into_pair(self) -> (stream_message::Model, Option<StreamMessageUser>) {
    let user = match (
      self.user_twitch_id,
      self.user_login_name,
      self.user_display_name,
    ) {
      (Some(twitch_id), Some(login_name), Some(display_name)) => Some(StreamMessageUser {
        twitch_id,
        login_name,
        display_name,
      }),
      _ => None,
    };

    let message = stream_message::Model {
      id: self.id,
      is_first_message: self.is_first_message,
      timestamp: self.timestamp,
      emote_only: self.emote_only,
      contents: self.contents,
      twitch_user_id: self.twitch_user_id,
      channel_id: self.channel_id,
      stream_id: self.stream_id,
      is_subscriber: self.is_subscriber,
      origin_id: self.origin_id,
      is_from_subscription_message: self.is_from_subscription_message,
    };
    (message, user)
  }
}
