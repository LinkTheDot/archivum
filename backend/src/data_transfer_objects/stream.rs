use crate::error::AppError;
use entities::*;
use entity::prelude::DateTimeUtc;
use sea_orm::*;

#[derive(Debug, serde::Serialize)]
pub struct StreamDto {
  pub id: i32,
  pub twitch_stream_id: u64,
  pub start_timestamp: Option<DateTimeUtc>,
  pub end_timestamp: Option<DateTimeUtc>,
  pub twitch_user: twitch_user::Model,
}

#[derive(Debug, serde::Serialize)]
pub struct StreamResponse {
  pub user: twitch_user::Model,
  pub streams: Vec<StreamListItem>,
}

#[derive(Debug, serde::Serialize)]
pub struct StreamListItem {
  pub id: i32,
  pub twitch_stream_id: u64,
  pub start_timestamp: Option<DateTimeUtc>,
  pub end_timestamp: Option<DateTimeUtc>,
  pub twitch_vod_id: Option<String>,
  pub title: Option<String>,
  pub muted_vod_segments: Vec<MutedVodSegmentResponse>,
}

#[derive(Debug, serde::Serialize)]
pub struct MutedVodSegmentResponse {
  /// Formatted as `hh:mm:ss`
  start: String,
  /// In seconds.
  duration: i32,
}

impl StreamDto {
  pub async fn response_from_stream_list(
    user: &twitch_user::Model,
    streams: Vec<stream::Model>,
    database_connection: &DatabaseConnection,
  ) -> Result<StreamResponse, AppError> {
    let streams_with_muted_segments =
      Self::get_muted_segments(streams, database_connection).await?;

    let filtered_streams = streams_with_muted_segments.into_iter().filter_map(|(stream, muted_vod_segments)| {
      if stream.twitch_user_id != user.id {
        tracing::warn!(
          "Encountered incorrect user ID when filtering for a stream response. Expected {} got {}",
          user.id,
          stream.twitch_user_id
        );

        return None;
      }

      let muted_vod_segments: Vec<MutedVodSegmentResponse> = muted_vod_segments.into_iter().map(Into::into).collect();

      Some(StreamListItem {
        id: stream.id,
        twitch_stream_id: stream.twitch_stream_id,
        start_timestamp: stream.start_timestamp,
        end_timestamp: stream.end_timestamp,
        twitch_vod_id: stream.twitch_vod_id,
        title: stream.title,
        muted_vod_segments
      })
    }).collect();

    Ok(StreamResponse {
      user: user.clone(),
      streams: filtered_streams,
    })
  }

  async fn get_muted_segments(
    streams: Vec<stream::Model>,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<(stream::Model, Vec<muted_vod_segment::Model>)>, AppError> {
    let muted_vod_segments = streams
      .load_many(muted_vod_segment::Entity, database_connection)
      .await?;

    Ok(streams.into_iter().zip(muted_vod_segments).collect())
  }

  pub async fn from_stream(
    stream: stream::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Self, AppError> {
    let Some(user) = twitch_user::Entity::find_by_id(stream.twitch_user_id)
      .one(database_connection)
      .await?
    else {
      return Err(AppError::CouldNotFindUserByTwitchId {
        user_id: stream.twitch_user_id.to_string(),
      });
    };

    Ok(Self {
      id: stream.id,
      twitch_stream_id: stream.twitch_stream_id,
      start_timestamp: stream.start_timestamp,
      end_timestamp: stream.end_timestamp,
      twitch_user: user,
    })
  }
}

impl From<muted_vod_segment::Model> for MutedVodSegmentResponse {
  fn from(muted_vod_segment: muted_vod_segment::Model) -> Self {
    let start_time = MutedVodSegmentResponse::seconds_to_time_string(muted_vod_segment.offset);

    MutedVodSegmentResponse {
      start: start_time,
      duration: muted_vod_segment.duration,
    }
  }
}

impl MutedVodSegmentResponse {
  /// Takes some amount of seconds and returns "hh::mm::ss"
  fn seconds_to_time_string(seconds: i32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
  }
}
