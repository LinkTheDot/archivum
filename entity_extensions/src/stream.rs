use crate::{
  errors::EntityExtensionError,
  stream::twitch_stream_response::{TwitchStreamData, TwitchStreamResponse},
};
use app_config::AppConfig;
use app_config::secret_string::Secret;
use entities::{muted_vod_segment, stream, twitch_user};
use reqwest::RequestBuilder;
use sea_orm::{sea_query::OnConflict, *};
use url::Url;

pub mod twitch_stream_response;

const HELIX_STREAM_QUERY_URL: &str = "https://api.twitch.tv/helix/streams";

pub trait StreamExtensions {
  fn is_live(&self) -> bool;
  /// Returns a stream model if the user is currently known to be streaming.
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
  ) -> Result<Vec<TwitchStreamData>, EntityExtensionError>
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
  async fn from_twitch_id(
    twitch_stream_id: u64,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<Self>, DbErr>
  where
    Self: Sized;
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

  /// Returns the list of stream data for currently active livestreams
  async fn get_active_livestreams<'a, I>(
    channels: I,
  ) -> Result<Vec<TwitchStreamData>, EntityExtensionError>
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
    let livestream_objects: TwitchStreamResponse = serde_json::from_str(&response_body)?;

    Ok(livestream_objects.data)
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
      .map(|muted_vod_segment| muted_vod_segment::ActiveModel {
        stream_id: Set(self.id),
        ..muted_vod_segment.into()
      })
      .collect();

    if muted_vod_segments.is_empty() {
      return Ok(());
    }

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

  async fn from_twitch_id(
    twitch_stream_id: u64,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<Self>, DbErr>
  where
    Self: Sized,
  {
    stream::Entity::find()
      .filter(stream::Column::TwitchStreamId.eq(twitch_stream_id))
      .one(database_connection)
      .await
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
