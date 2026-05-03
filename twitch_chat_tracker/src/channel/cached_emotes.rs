use crate::{channel::third_party_emote_list::EmoteList, errors::AppError};
use entities::{channel_emote_cache, emote, twitch_user};
use sea_orm::{sea_query::OnConflict, *};
use std::collections::HashMap;

pub struct EmoteCache;

#[derive(Debug)]
pub enum CacheNameIdentifier<'a> {
  TwitchUser(&'a twitch_user::Model),
  Global,
}

struct CacheDifference<'a> {
  missing_emote_list: Vec<&'a emote::Model>,
  new_emote_list: Vec<channel_emote_cache::ActiveModel>,
}

impl EmoteCache {
  pub const GLOBAL_EMOTE_CACHE_ID: i32 = 0;

  /// Checks the cached emotes for the given channel and updates it to match the given emote list.
  pub async fn update_cache(
    identifier: CacheNameIdentifier<'_>,
    emote_list: &EmoteList,
    database_connection: &DatabaseConnection,
  ) -> Result<(), AppError> {
    tracing::info!("Updating emote cache for (identifier:?)");
    let channel_id = match identifier {
      CacheNameIdentifier::TwitchUser(channel) => channel.id,
      CacheNameIdentifier::Global => Self::GLOBAL_EMOTE_CACHE_ID,
    };
    let existing_cache: HashMap<String, emote::Model> =
      Self::get_cached_emotes(channel_id, database_connection)
        .await?
        .into_iter()
        .map(|emote| (emote.name.clone(), emote))
        .collect();

    let mut new_emote_list: Vec<channel_emote_cache::ActiveModel> = vec![];
    let missing_emote_list: Vec<&emote::Model> = existing_cache
      .iter()
      .filter_map(|(emote_name, emote)| {
        if !emote_list.contains(emote_name) {
          Some(emote)
        } else {
          let cached_emote = channel_emote_cache::ActiveModel {
            twitch_user_id: Set(channel_id),
            emote_id: Set(emote.id),
          };

          new_emote_list.push(cached_emote);

          None
        }
      })
      .collect();

    Self::insert_new_emotes(new_emote_list, database_connection).await?;
    Self::remove_missing_emotes(missing_emote_list, channel_id, database_connection).await?;

    Ok(())
  }

  pub async fn retrieve_from_cache(
    identifier: CacheNameIdentifier<'_>,
    database_connection: &DatabaseConnection,
  ) -> Result<EmoteList, AppError> {
    tracing::info!("Retrieving emote cache for {identifier:?}");
    let (channel_id, channel_name) = match identifier {
      CacheNameIdentifier::TwitchUser(channel) => (channel.id, channel.login_name.as_str()),
      CacheNameIdentifier::Global => (Self::GLOBAL_EMOTE_CACHE_ID, EmoteList::GLOBAL_NAME),
    };
    let cached_emotes = Self::get_cached_emotes(channel_id, database_connection).await?;

    Ok(EmoteList::from_list(
      channel_name.to_string(),
      cached_emotes,
    ))
  }

  async fn insert_new_emotes(
    new_emote_list: Vec<channel_emote_cache::ActiveModel>,
    database_connection: &DatabaseConnection,
  ) -> Result<(), AppError> {
    if new_emote_list.is_empty() {
      tracing::info!("No new emotes, skipping.");

      return Ok(());
    }

    let emote_count = new_emote_list.len();
    tracing::info!("Inserting {emote_count} new emotes into cache.");
    let potentional_conflicting_columns = [
      channel_emote_cache::Column::TwitchUserId,
      channel_emote_cache::Column::EmoteId,
    ];
    let _ = channel_emote_cache::Entity::insert_many(new_emote_list)
      .on_conflict(
        OnConflict::columns(potentional_conflicting_columns)
          .do_nothing_on(potentional_conflicting_columns)
          .to_owned(),
      )
      .exec(database_connection)
      .await?;

    Ok(())
  }

  async fn remove_missing_emotes(
    remove_emote_list: Vec<&emote::Model>,
    channel_id: i32,
    database_connection: &DatabaseConnection,
  ) -> Result<(), AppError> {
    if remove_emote_list.is_empty() {
      tracing::info!("No missing emotes, skipping.");

      return Ok(());
    }

    let emote_count = remove_emote_list.len();
    tracing::info!("Removing {emote_count} missing emotes from cache for channel_id: {channel_id}");
    let emote_id_list: Vec<i32> = remove_emote_list.iter().map(|emote| emote.id).collect();

    channel_emote_cache::Entity::delete_many()
      .filter(channel_emote_cache::Column::TwitchUserId.eq(channel_id))
      .filter(channel_emote_cache::Column::EmoteId.is_in(emote_id_list))
      .exec(database_connection)
      .await?;

    Ok(())
  }

  async fn get_cached_emotes(
    channel_id: i32,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<emote::Model>, AppError> {
    tracing::info!("Retrieving existing cache for channel_id {channel_id}.");

    emote::Entity::find()
      .inner_join(channel_emote_cache::Entity)
      .filter(channel_emote_cache::Column::TwitchUserId.eq(channel_id))
      .all(database_connection)
      .await
      .map_err(Into::into)
  }

  fn difference_in_cache(
    existing_cache: HashMap<String, emote::Model>,
    emote_list: &EmoteList,
  ) -> CacheDifference {
    // let new_emote_list: Vec<channel_emote_cache::ActiveModel> = existing_cache
    //   .iter()
    //   .filter_map(|(emote_name, emote)| {
    //     if emote_list.contains(emote_name) {
    //       return None;
    //     } else {
    //       let cached_emote = channel_emote_cache::ActiveModel {
    //         twitch_user_id: Set(channel_id),
    //         emote_id: Set(emote.id),
    //       };
    //
    //       Some(cached_emote)
    //     }
    //   })
    //   .collect();
    todo!()
  }
}
