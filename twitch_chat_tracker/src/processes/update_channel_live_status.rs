use crate::channel::tracked_channels::TrackedChannels;
use crate::{errors::AppError, websocket_connection::config::TwitchWebsocketConfig};
use database_connection::get_database_connection;
use entities::stream;
use entity_extensions::stream::StreamExtensions;
use sea_orm::*;
use sea_query::OnConflict;

const TIMEOUT_COUNT_UNTIL_RESET: usize = 5;

pub async fn update_channel_live_statuses(tracked_channels: TrackedChannels) -> ! {
  tracing::info!("Starting channel status update process.");
  let database_connection = get_database_connection().await;

  tracing::info!("Checking for active livestreams.");
  if let Err(error) = update_active_streams(&tracked_channels, database_connection).await {
    tracing::error!(
      "Failed to update active livestreams from the tracked channels list. Reason: {}",
      error
    );
  }

  let mut websocket_config = TwitchWebsocketConfig::new(tracked_channels, database_connection)
    .await
    .unwrap();

  let mut timedout_count = 0;

  tracing::info!("Running channel status update process.");

  loop {
    match websocket_config.check_for_stream_message().await {
      Err(AppError::WebsocketTimeout) => {
        tracing::error!("{}", AppError::WebsocketTimeout);

        restart_connection(&mut websocket_config, database_connection).await;
      }

      Err(AppError::TungsteniteError(tungstenite::error::Error::Io(error))) => {
        tracing::error!("Received a fatal IO error: {:?}.", error);

        restart_connection(&mut websocket_config, database_connection).await;
      }

      Err(error) => {
        tracing::error!("Failed to process a websocket message: {}", error);
      }

      Ok(false) => {
        timedout_count += 1;

        tracing::info!("No message was received.");

        if timedout_count >= TIMEOUT_COUNT_UNTIL_RESET {
          restart_connection(&mut websocket_config, database_connection).await;
        } else {
          continue;
        }
      }

      Ok(_) => (),
    }

    timedout_count = 0;
  }
}

/// Attempts to restart the websocket connection.
///
/// Exits the program with an error if the connection could not be re-established.
async fn restart_connection(
  websocket_config: &mut TwitchWebsocketConfig,
  database_connection: &DatabaseConnection,
) {
  if let Err(error) = websocket_config.restart(database_connection).await {
    tracing::error!(
      "Failed to restart the websocket config. Reason: {}. Exiting the program",
      error
    );

    std::process::exit(1);
  }
}

async fn update_active_streams(
  tracked_channels: &TrackedChannels,
  database_connection: &DatabaseConnection,
) -> Result<(), AppError> {
  let channels = tracked_channels.all_channels();
  let current_live_channels = stream::Model::get_active_livestreams(channels).await?;
  let mut live_stream_active_models: Vec<stream::ActiveModel> = vec![];

  for stream_data in current_live_channels {
    let Ok(stream_twitch_id) = stream_data.stream_id.parse::<u64>() else {
      tracing::error!(
        "Failed to parse a stream ID. Streamer: {:?}. Value: {:?}",
        stream_data.user_login,
        stream_data.stream_id
      );

      continue;
    };
    let Some(streamer) = tracked_channels.get_channel(&stream_data.user_login) else {
      tracing::error!(
        "Failed to find streamer {:?} in the tracked channels list when updating active streams.",
        stream_data.user_login
      );

      continue;
    };

    let active_model = stream::ActiveModel {
      twitch_stream_id: ActiveValue::Set(stream_twitch_id),
      start_timestamp: ActiveValue::Set(Some(stream_data.started_at)),
      twitch_user_id: ActiveValue::Set(streamer.id),
      ..Default::default()
    };

    live_stream_active_models.push(active_model);
  }

  stream::Entity::insert_many(live_stream_active_models)
    .on_conflict(
      OnConflict::column(stream::Column::TwitchStreamId)
        .do_nothing_on([stream::Column::TwitchStreamId])
        .to_owned(),
    )
    .do_nothing()
    .exec(database_connection)
    .await?;

  Ok(())
}
