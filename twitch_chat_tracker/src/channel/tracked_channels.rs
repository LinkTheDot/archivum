use crate::errors::AppError;
use app_config::AppConfig;
use database_connection::get_database_connection;
use entities::{prelude::*, twitch_user};
use entity_extensions::{prelude::*, twitch_user::ChannelIdentifier};
use sea_orm::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TrackedChannels {
  /// [`login_name`](entities::twitch_user::Model::login_name) is used as the key.
  channels: HashMap<String, twitch_user::Model>,
}

impl TrackedChannels {
  pub async fn new() -> Result<Self, AppError> {
    let connected_channels = Self::get_channels_from_list(AppConfig::channels()).await?;

    Ok(TrackedChannels {
      channels: connected_channels,
    })
  }

  #[cfg(test)]
  pub fn new_from_map(channels: HashMap<String, twitch_user::Model>) -> Self {
    Self { channels }
  }

  pub fn get_channel(&self, channel_login: &str) -> Option<twitch_user::Model> {
    self.channels.get(channel_login).cloned()
  }

  pub fn get_channel_by_twitch_id(&self, twitch_id: i32) -> Option<twitch_user::Model> {
    self
      .channels
      .iter()
      .find_map(|(_name, channel)| (channel.twitch_id == twitch_id).then_some(channel.to_owned()))
  }

  pub fn all_channels(&self) -> Vec<&twitch_user::Model> {
    self.channels.values().collect()
  }

  /// Takes a list of channel login names and returns a map containing the <login_name, channel_data>.
  async fn get_channels_from_list(
    channels: &Vec<String>,
  ) -> Result<HashMap<String, twitch_user::Model>, AppError> {
    let database_connection = get_database_connection().await;
    let existing_channels_in_database: Vec<twitch_user::Model> = TwitchUser::find()
      .filter(twitch_user::Column::LoginName.is_in(channels))
      .all(database_connection)
      .await?;
    let channel_map_from_database: HashMap<String, twitch_user::Model> =
      existing_channels_in_database
        .into_iter()
        .map(|channel| (channel.login_name.clone(), channel))
        .collect();

    let channels_missing_from_database: Vec<ChannelIdentifier<&str>> = channels
      .iter()
      .filter_map(|channel_name| {
        (!channel_map_from_database.contains_key(channel_name))
          .then_some(ChannelIdentifier::Login(channel_name.as_str()))
      })
      .collect();

    if channels_missing_from_database.is_empty() {
      return Ok(channel_map_from_database);
    }

    tracing::info!(
      "Adding missing channels from database: {:?}",
      channels_missing_from_database
    );

    let channels_missing_from_database_active_models =
      twitch_user::Model::query_helix_for_channels_from_list(&channels_missing_from_database)
        .await?;
    let _insert_result = TwitchUser::insert_many(channels_missing_from_database_active_models)
      .exec(database_connection)
      .await?;
    let missing_channels: Vec<&str> = channels_missing_from_database
      .into_iter()
      .map(Into::into)
      .collect();

    let missing_channels_from_database = TwitchUser::find()
      .filter(twitch_user::Column::LoginName.is_in(missing_channels))
      .all(database_connection)
      .await?
      .into_iter()
      .map(|channel| (channel.login_name.clone(), channel));

    Ok(
      channel_map_from_database
        .into_iter()
        .chain(missing_channels_from_database)
        .collect(),
    )
  }
}
