use database_connection::get_database_connection;
use entities::twitch_user;
use sea_orm::*;
use std::time::Duration;
use twitch_chat_tracker::errors::AppError;

use crate::user_chat_months::UserChatMonths;

pub struct SpanixScrubberConfig {
  pub channel: twitch_user::Model,
  pub database_connection: &'static DatabaseConnection,
  pub user_chat_months: UserChatMonths,
}

impl SpanixScrubberConfig {
  pub const USER_ITERATION_TIME_LIMIT: Duration = Duration::new(1, 0);
  pub const REMOVE_AFTER_YEAR: i32 = 2025;
  pub const REMOVE_AFTER_MONTH: i32 = 1;
  pub const DATA_OUTPUT_DIRECTORY: &str = "spanix_scrubber/spanix-user-messages";
  pub const FAILED_MESSAGES_OUTPUT_DIRECTORY: &str = "spanix_scrubber/failed_spanix_messages";
  pub const FAILED_MESSAGES_FILE_NAME: &str = "{data_set}-failed_spanix_messages.dat";
  pub const END_OF_FILE_INDICATOR: &str = "==EOF==";

  pub async fn new(channel_login: &str) -> Result<Self, AppError> {
    let database_connection = get_database_connection().await;

    let Some(channel) = twitch_user::Entity::find()
      .filter(twitch_user::Column::LoginName.eq(channel_login))
      .one(database_connection)
      .await?
    else {
      return Err(AppError::UserDoesNotExist(channel_login.to_string()));
    };
    let user_chat_months =
      UserChatMonths::retrieve_for_channel(&channel, database_connection).await?;

    Ok(Self {
      channel,
      user_chat_months,
      database_connection,
    })
  }
}
