use crate::error::AppError;
use entities::stream;
use sea_orm::*;

pub trait GetStream {
  fn get_stream_id(&self) -> Option<&str>;
  fn get_twitch_stream_id(&self) -> Option<&str>;
  fn get_twitch_vod_id(&self) -> Option<&str>;
}

pub async fn get_stream<Q: GetStream>(
  query: &Q,
  database_connection: &DatabaseConnection,
) -> Result<stream::Model, AppError> {
  if let Some(id_str) = query.get_stream_id() {
    let id: i32 = id_str
      .parse()
      .map_err(|_| AppError::InvalidStreamIdentifier { value: id_str.to_owned() })?;
    stream::Entity::find_by_id(id)
      .one(database_connection)
      .await?
      .ok_or(AppError::FailedToFindStreamByID { stream_id: id })
  } else if let Some(twitch_id_str) = query.get_twitch_stream_id() {
    let twitch_id: u64 = twitch_id_str
      .parse()
      .map_err(|_| AppError::InvalidStreamIdentifier { value: twitch_id_str.to_owned() })?;
    stream::Entity::find()
      .filter(stream::Column::TwitchStreamId.eq(twitch_id))
      .one(database_connection)
      .await?
      .ok_or(AppError::FailedToFindStreamByTwitchStreamId { twitch_stream_id: twitch_id })
  } else if let Some(vod_id) = query.get_twitch_vod_id() {
    stream::Entity::find()
      .filter(stream::Column::TwitchVodId.eq(vod_id))
      .one(database_connection)
      .await?
      .ok_or(AppError::FailedToFindStreamByVodId { vod_id: vod_id.to_owned() })
  } else {
    Err(AppError::NoStreamIdentifierProvided)
  }
}
