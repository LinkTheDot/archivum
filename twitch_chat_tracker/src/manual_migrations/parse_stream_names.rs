use database_connection::get_database_connection;
use entities::stream;
use sea_orm::*;
use std::collections::HashMap;
use tokio::fs;
use twitch_chat_tracker::errors::AppError;

#[allow(dead_code)]
/// Parses a file of format "stream_id\tstream_name" where \t is tab.
pub async fn parse_stream_names_from_file(file_path: &str) -> ! {
  if let Err(error) = run(file_path).await {
    tracing::error!("Failed to parse stream names from file. Reason: `{error}`");

    std::process::exit(1)
  }

  std::process::exit(0)
}

async fn run(file_path: &str) -> Result<(), AppError> {
  tracing::info!("Getting file at {file_path}.");
  let file = fs::read_to_string(file_path).await?;
  tracing::info!("Got file.");
  let database_connection = get_database_connection().await;

  let stream_twitch_ids_and_titles: HashMap<u64, String> = file
    .lines()
    .filter_map(|line| {
      let mut parts = line.splitn(2, "\t");
      let Some(stream_id) = parts.next() else {
        tracing::error!("Missing stream_id from line {line}");
        return None;
      };
      let Ok(stream_id) = stream_id.parse::<u64>() else {
        tracing::error!("Failed to parse stream_id from line {line}");
        return None;
      };
      let stream_name: String = parts.collect();

      Some((stream_id, stream_name))
    })
    .collect();
  tracing::info!(
    "Got {} unique streams and titles.",
    stream_twitch_ids_and_titles.len()
  );
  let stream_ids: Vec<u64> = stream_twitch_ids_and_titles.keys().cloned().collect();
  let streams = stream::Entity::find()
    .filter(stream::Column::TwitchStreamId.is_in(stream_ids))
    .all(database_connection)
    .await?;

  tracing::info!("Got {} streams.", streams.len());

  for stream in streams {
    let stream_id = stream.id;
    let Some(stream_title) = stream_twitch_ids_and_titles.get(&stream.twitch_stream_id) else {
      tracing::error!("Missing item {stream:?}");
      continue;
    };

    let updated_model = stream::ActiveModel {
      title: Set(Some(stream_title.to_owned())),
      ..stream.into_active_model()
    };

    if let Err(error) = updated_model.update(database_connection).await {
      tracing::error!(
        "failed to update stream of ID {}. Reason: `{error}`",
        stream_id
      );
    }
  }

  Ok(())
}
