use crate::checks::update_vod_data::{
  config::vod_stream_pair::{VodAndStream, VodStreamPairs},
  twitch_objects::vod_response::TwitchVodResponse,
};
use anyhow::anyhow;
use app_config::{AppConfig, secret_string::Secret};
use chrono::{Duration, Utc};
use database_connection::get_database_connection;
use entities::{stream, twitch_user};
use entity_extensions::stream::StreamExtensions;
use sea_orm::*;
use std::collections::{HashMap, HashSet};
use url::Url;

mod vod_stream_pair;

const VOD_AGE_DAYS_MAX_RANGE: (usize, usize) = (1, 60);
const HELIX_VOD_QUERY_URL: &str = "https://api.twitch.tv/helix/videos";

pub struct UpdateVodDataConfig<'a> {
  /// How many days old should vods be to be checked.
  max_vod_age_days: usize,
  database_connection: &'a DatabaseConnection,
}

impl UpdateVodDataConfig<'_> {
  /// Takes how many days back to check for vods.
  ///
  /// Clamped to 1-60.
  pub async fn new(vod_age_days: usize) -> Self {
    let database_connection = get_database_connection().await;
    let vod_age_days = vod_age_days.clamp(VOD_AGE_DAYS_MAX_RANGE.0, VOD_AGE_DAYS_MAX_RANGE.1);

    Self {
      max_vod_age_days: vod_age_days,
      database_connection,
    }
  }

  pub async fn run(self) -> anyhow::Result<()> {
    let streams = self.get_streams().await?;
    let users = self.get_users_from_stream_list(&streams).await?;

    let vods = Self::query_for_vods_from_users(&users, &streams).await;
    let vod_stream_pairs = Self::build_vod_stream_pairs(users, streams, vods).await;

    self.update_streams_with_vod_data(vod_stream_pairs).await;

    Ok(())
  }

  async fn get_streams(&self) -> anyhow::Result<Vec<stream::Model>> {
    let now = Utc::now();
    let subtract_days = Duration::days(self.max_vod_age_days as i64);
    let earliest_stream_date = (now - subtract_days).date_naive();

    tracing::info!("Getting streams as early as {}", earliest_stream_date);

    let streams = stream::Entity::find()
      .filter(stream::Column::StartTimestamp.gte(earliest_stream_date))
      .all(self.database_connection)
      .await?;

    tracing::info!("Got {} streams to process.", streams.len());

    Ok(streams)
  }

  async fn get_users_from_stream_list(
    &self,
    stream_list: &[stream::Model],
  ) -> anyhow::Result<Vec<twitch_user::Model>> {
    tracing::info!("Getting users from stream_list.");
    let user_ids: HashSet<i32> = stream_list
      .iter()
      .map(|stream| stream.twitch_user_id)
      .collect();

    tracing::info!("Retrieving {} unique users.", user_ids.len());

    twitch_user::Entity::find()
      .filter(twitch_user::Column::Id.is_in(user_ids))
      .all(self.database_connection)
      .await
      .map_err(Into::into)
  }

  async fn query_for_vods_from_users(
    users: &[twitch_user::Model],
    desired_streams: &[stream::Model],
  ) -> HashMap<i32, TwitchVodResponse> {
    tracing::info!("Getting vods for users list.");
    let desired_stream_ids: HashSet<String> = desired_streams
      .iter()
      .map(|stream| stream.twitch_stream_id.to_string())
      .collect();
    let mut users_and_vods = HashMap::new();

    for user in users {
      let mut vod_response = match Self::get_vods_for_user(user).await {
        Ok(vod_response) => vod_response,
        Err(error) => {
          tracing::error!("Failed to get vods for user `{user:?}`. Reason: `{error}`");
          continue;
        }
      };

      tracing::info!("Filtering unwanted vods");

      vod_response
        .vod_list
        .retain(|vod| {
          let Some(stream_id) = vod.stream_id() else {
            return false;
          };

          desired_stream_ids.contains(stream_id)
        });

      tracing::info!(
        "`{}` vods left after filtering.",
        vod_response.vod_list.len()
      );

      if !vod_response.vod_list.is_empty() {
        users_and_vods.insert(user.id, vod_response);
      } else {
        tracing::info!("Skipping empty vod list.");
      }

    }

    users_and_vods
  }

  async fn get_vods_for_user(user: &twitch_user::Model) -> anyhow::Result<TwitchVodResponse> {
    tracing::info!("Getting vods for `{}`-`{}`", user.id, user.login_name);
    let mut query_url = Url::parse(HELIX_VOD_QUERY_URL)?;
    let reqwest_client = reqwest::Client::new();

    {
      let mut query_pairs = query_url.query_pairs_mut();

      query_pairs.append_pair("user_id", &user.twitch_id.to_string());
    }

    let request = reqwest_client
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
      );

    let response = request.send().await?;
    let status = response.status();

    if !status.is_success() {
      return Err(anyhow!(
        "Failed to get vod response. Error code `{}`",
        status.as_u16()
      ));
    }

    let response_body = response.text().await?;

    tracing::debug!("{response_body}");

    serde_json::from_str(&response_body).map_err(Into::into)
  }

  async fn build_vod_stream_pairs(
    users: Vec<twitch_user::Model>,
    streams: Vec<stream::Model>,
    mut vods: HashMap<i32, TwitchVodResponse>,
  ) -> Vec<VodStreamPairs> {
    let mut streams: HashMap<String, stream::Model> = streams
      .into_iter()
      .map(|stream| (stream.twitch_stream_id.to_string(), stream))
      .collect();

    users
      .into_iter()
      .filter_map(|user| {
        let Some(vods) = vods.remove(&user.id) else {
          tracing::warn!("No vods found for user `{user:?}`");
          return None;
        };

        let vods_and_streams: Vec<VodAndStream> = vods
          .vod_list
          .into_iter()
          .filter_map(|vod| {
            let stream_id = vod.stream_id()?;
            let Some(stream) = streams.remove(stream_id) else {
              tracing::error!("Failed to get stream {stream_id} from list.");
              return None;
            };

            Some(VodAndStream { vod, stream })
          })
          .collect();

        Some(VodStreamPairs {
          _user: user,
          vods_and_streams,
        })
      })
      .collect()
  }

  async fn update_streams_with_vod_data(&self, vod_stream_pairs: Vec<VodStreamPairs>) {
    for VodStreamPairs {
      vods_and_streams, ..
    } in vod_stream_pairs
    {
      for VodAndStream { vod, stream } in vods_and_streams {
        let stream_id = stream.id;
        let existing_stream = stream.into_active_model();
        let updated_stream_active_model = stream::ActiveModel {
          title: Set(Some(vod.vod_title().to_string())),
          twitch_vod_id: Set(Some(vod.vod_id().to_string())),
          ..existing_stream
        };

        let result = updated_stream_active_model
          .update(self.database_connection)
          .await;

        let stream = match result {
          Ok(stream_model) => stream_model,
          Err(error) => {
            tracing::error!("Failed to update stream of ID `{stream_id}`. Reason: `{error}`");
            continue;
          }
        };

        let result = stream
          .insert_muted_segments(self.database_connection, vod.muted_segments())
          .await;

        if let Err(error) = result {
          tracing::error!(
            "Failed to insert muted segments for stream of ID `{}`. Reason: `{error}`",
            stream.id
          );
          continue;
        }
      }
    }
  }
}
