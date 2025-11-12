use crate::errors::EntityExtensionError;
use app_config::AppConfig;
use app_config::secret_string::Secret;
use chrono::{DateTime, Utc};
use entities::{muted_vod_segment, stream, twitch_user};
use reqwest::RequestBuilder;
use sea_orm::{sea_query::OnConflict, *};
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

const HELIX_STREAM_QUERY_URL: &str = "https://api.twitch.tv/helix/streams";

pub trait StreamExtensions {
  fn is_live(&self) -> bool;
  /// Returns a stream object if the user passed in is currently streaming.
  async fn get_active_stream_for_user(
    user: &twitch_user::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<stream::Model>, DbErr>;
  async fn get_stream_from_stream_twitch_id(
    stream_twitch_id: u64,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<stream::Model>, DbErr>;
  /// Returns a map of login_name: (stream_start, stream_twitch_id)
  async fn get_active_livestreams<'a, I>(
    channels: I,
  ) -> Result<HashMap<String, (DateTime<Utc>, String)>, EntityExtensionError>
  where
    I: IntoIterator<Item = &'a twitch_user::Model>;
  async fn insert_muted_segments<I, M>(
    &self,
    database_connection: &DatabaseConnection,
    muted_vod_segments: I,
  ) -> Result<(), DbErr>
  where
    I: IntoIterator<Item = M>,
    M: Into<muted_vod_segment::ActiveModel>;
}

impl StreamExtensions for stream::Model {
  fn is_live(&self) -> bool {
    self.end_timestamp.is_none()
  }

  async fn get_active_stream_for_user(
    user: &twitch_user::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<stream::Model>, DbErr> {
    // Fetch the latest stream for the given user
    let latest_stream = stream::Entity::find()
      .filter(stream::Column::TwitchUserId.eq(user.id))
      .order_by_desc(stream::Column::StartTimestamp)
      .one(database_connection)
      .await?;

    Ok(latest_stream.filter(stream::Model::is_live))
  }

  async fn get_stream_from_stream_twitch_id(
    stream_twitch_id: u64,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<stream::Model>, DbErr> {
    stream::Entity::find()
      .filter(stream::Column::TwitchStreamId.eq(stream_twitch_id))
      .one(database_connection)
      .await
  }

  /// Returns a map of login_name: (stream_start, stream_twitch_id)
  async fn get_active_livestreams<'a, I>(
    channels: I,
  ) -> Result<HashMap<String, (DateTime<Utc>, String)>, EntityExtensionError>
  where
    I: IntoIterator<Item = &'a twitch_user::Model>,
  {
    let request = build_get_streams_request(channels).await?;
    let response = request.send().await?;

    let status = response.status();

    if !status.is_success() {
      return Err(EntityExtensionError::FailedResponse {
        location: "get active livestreams",
        code: status.as_u16(),
      });
    }

    let response_body = response.text().await?;
    let Value::Object(response_value): Value = serde_json::from_str(&response_body)? else {
      return Err(EntityExtensionError::UnknownResponseBody {
        location: "get active livestreams update live status",
        response: response_body,
      });
    };
    let Some(Value::Array(live_streams)) = response_value.get("data") else {
      return Err(EntityExtensionError::UnknownResponseBody {
        location: "get active livestreams update live status live stream list",
        response: response_body,
      });
    };

    let mut live_channels: HashMap<String, (DateTime<Utc>, String)> = HashMap::new();

    for live_stream in live_streams {
      let Value::Object(live_stream) = live_stream else {
        continue;
      };

      let Some(Value::String(live_status)) = live_stream.get("type") else {
        continue;
      };

      if live_status != "live" {
        continue;
      }

      let Some(Value::String(streamer_login_name)) = live_stream.get("user_login") else {
        continue;
      };
      let Some(Value::String(stream_start)) = live_stream.get("started_at") else {
        continue;
      };
      let stream_start = match stream_start.parse::<DateTime<Utc>>() {
        Ok(stream_start) => stream_start,
        Err(error) => {
          tracing::error!(
            "Failed to parse the date time for streamer {:?}. Reason: {:?}",
            streamer_login_name,
            error
          );
          continue;
        }
      };
      let Some(Value::String(stream_id)) = live_stream.get("id") else {
        tracing::error!(
          "Failed to get livestream ID for channel `{:?}`",
          streamer_login_name
        );
        continue;
      };

      live_channels.insert(
        streamer_login_name.to_owned(),
        (stream_start, stream_id.to_owned()),
      );
    }

    Ok(live_channels)
  }

  async fn insert_muted_segments<I, M>(
    &self,
    database_connection: &DatabaseConnection,
    muted_vod_segments: I,
  ) -> Result<(), DbErr>
  where
    I: IntoIterator<Item = M>,
    M: Into<muted_vod_segment::ActiveModel>,
  {
    let muted_vod_segments: Vec<muted_vod_segment::ActiveModel> = muted_vod_segments
      .into_iter()
      .map(|muted_vod_segment| {
        let mut muted_vod_segment: muted_vod_segment::ActiveModel = muted_vod_segment.into();

        muted_vod_segment.stream_id = Set(self.id);

        muted_vod_segment
      })
      .collect();

    if muted_vod_segments.is_empty() {
      return Ok(());
    }

    tracing::info!("{muted_vod_segments:?}");

    let potentional_conflicting_columns = [
          muted_vod_segment::Column::StreamId,
          muted_vod_segment::Column::Offset,
    ];

    let _insert_result = muted_vod_segment::Entity::insert_many(muted_vod_segments)
      .on_conflict(
        OnConflict::columns(potentional_conflicting_columns)
        .do_nothing_on(potentional_conflicting_columns)
        .to_owned(),
      )
      .exec(database_connection)
      .await?;

    Ok(())
  }
}

/// Takes the list of channels and builds the request for querying streams.
async fn build_get_streams_request<'a, I>(
  channels: I,
) -> Result<RequestBuilder, EntityExtensionError>
where
  I: IntoIterator<Item = &'a twitch_user::Model>,
{
  let mut query_url = Url::parse(HELIX_STREAM_QUERY_URL)?;
  let reqwest_client = reqwest::Client::new();

  query_url.query_pairs_mut().append_pair("first", "100");

  for channel_data in channels {
    query_url
      .query_pairs_mut()
      .append_pair("user_login", &channel_data.login_name);
  }

  Ok(
    reqwest_client
      .get(query_url)
      .header(
        "Authorization",
        format!(
          "Bearer {}",
          Secret::read_secret_string(AppConfig::access_token().read_value())
        ),
      )
      .header(
        "Client-Id",
        Secret::read_secret_string(AppConfig::client_id().read_value()),
      ),
  )
}
