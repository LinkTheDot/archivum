use crate::error::AppError;
use entities::{raid, stream, twitch_user};
use sea_orm::{prelude::DateTimeUtc, *};
use std::collections::HashMap;

#[derive(Debug, serde::Serialize)]
pub struct RaidDto {
  pub id: i32,
  pub raider: Option<twitch_user::Model>,
  pub timestamp: DateTimeUtc,
  pub viewers_from_raid: i32,
  pub stream_title: Option<String>,
}

impl RaidDto {
  /// Gets the list of raids to a given user with their related stream titles and raiders.
  ///
  /// Optionally takes a raider to only look for raids of a given user.
  pub async fn from_raid_list(
    raids: Vec<raid::Model>,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<Self>, AppError> {
    let related_streams = raids.load_one(stream::Entity, database_connection).await?;
    let raiders = twitch_user::Entity::find()
      .filter(
        twitch_user::Column::Id.is_in(raids.iter().filter_map(|raid| raid.raider_twitch_user_id)),
      )
      .all(database_connection)
      .await?;
    let raiders: HashMap<i32, twitch_user::Model> =
      raiders.into_iter().map(|user| (user.id, user)).collect();

    Ok(
      raids
        .into_iter()
        .zip(related_streams)
        .map(|(raid, stream)| {
          let stream_title = stream.and_then(|stream| stream.title);
          let raider = raid
            .raider_twitch_user_id
            .and_then(|id| raiders.get(&id).cloned());

          RaidDto {
            id: raid.id,
            raider,
            timestamp: raid.timestamp,
            viewers_from_raid: raid.size,
            stream_title,
          }
        })
        .collect(),
    )
  }
}
