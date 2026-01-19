use crate::channel::third_party_emote_list::EmoteList;
use crate::errors::AppError;
use entities::{emote, twitch_user};
use entity_extensions::{prelude::*, twitch_user::ChannelIdentifier};
use futures::stream::{self, StreamExt};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

#[derive(Debug)]
pub struct EmoteListStorage {
  third_party_emote_lists: HashMap<String, EmoteList>,
}

impl EmoteListStorage {
  /// This constant limits how many channels retrieve emote data from third parties at a time.
  const CHANNEL_FETCH_EMOTE_BATCH_LIMIT: usize = 2;

  /// Generates the list of emotes for each channel in the app config.
  /// Global emotes are under the name [`GLOBAL`](EmoteList::GLOBAL_NAME).
  ///
  /// If the emote list couldn't be retrieved for whatever reason, the name is still stored but with an empty list.
  pub async fn new(
    channel_names: &[String],
    database_connection: &DatabaseConnection,
  ) -> Result<Self, AppError> {
    if cfg!(test) {
      panic!("Called new on EmoteListStorage in a test environment. Use EmoteListStorage::test_list instead.");
    }

    let channel_names: Vec<&str> = channel_names.iter().map(String::as_str).collect();

    let channel_identifiers = ChannelIdentifier::from_login_list(channel_names);
    let channels =
      twitch_user::Model::get_many_by_identifier(channel_identifiers, database_connection).await?;

    let mut third_party_emote_lists_results: Vec<Result<(String, EmoteList), AppError>> =
      stream::iter(channels)
        .map(|channel| async move {
          let channel_login = channel.login_name.clone();
          let emote_list = EmoteList::get_list(&channel, database_connection).await?;

          Ok::<_, AppError>((channel_login, emote_list))
        })
        .buffer_unordered(Self::CHANNEL_FETCH_EMOTE_BATCH_LIMIT)
        .collect::<Vec<_>>()
        .await;

    let global_emote_list = Self::get_global_emote_list(database_connection).await;
    third_party_emote_lists_results.push(global_emote_list);

    let third_party_emote_lists: HashMap<String, EmoteList> = third_party_emote_lists_results
      .into_iter()
      .map(|result| {
        result.inspect_err(|error| {
          tracing::error!("Failed to retrieve emote list for channel. Error: {error}");
        })
      })
      .collect::<Result<_, AppError>>()?;

    Ok(Self {
      third_party_emote_lists,
    })
  }

  async fn get_global_emote_list(
    database_connection: &DatabaseConnection,
  ) -> Result<(String, EmoteList), AppError> {
    Ok((
      EmoteList::GLOBAL_NAME.to_string(),
      EmoteList::get_global_list(database_connection).await?,
    ))
  }

  /// Returns the list of emotes stored defined by EmoteList::TEST_EMOTES for every channel under AppConfig::TEST_CHANNELS and EmoteList::GLOBAL_NAME.
  ///
  /// None is returned if this method is called without the test flag set.
  pub fn test_list() -> Option<Self> {
    if !cfg!(test) {
      return None;
    }

    let test_emote_storage = EmoteList::get_test_list()?;

    let third_party_emote_lists = test_emote_storage.into_iter().fold(
      HashMap::new(),
      |mut third_party_emote_lists, emote_list| {
        third_party_emote_lists.insert(emote_list.channel_name().to_string(), emote_list);

        third_party_emote_lists
      },
    );

    Some(Self {
      third_party_emote_lists,
    })
  }

  pub fn get_channel_emote(
    &self,
    channel: &twitch_user::Model,
    emote_name: &str,
  ) -> Option<&emote::Model> {
    if let Some(channel_emote_list) = self.third_party_emote_lists.get(&channel.login_name) {
      if let Some(emote) = channel_emote_list.get(emote_name) {
        return Some(emote);
      }
    }

    if let Some(global_emote_list) = self.third_party_emote_lists.get(EmoteList::GLOBAL_NAME) {
      if let Some(emote) = global_emote_list.get(emote_name) {
        return Some(emote);
      }
    }

    None
  }

  pub fn contains_channel(&self, channel: &twitch_user::Model) -> bool {
    self
      .third_party_emote_lists
      .contains_key(&channel.login_name)
  }
}
