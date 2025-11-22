use crate::channel::active_streams::ActiveStreams;
use crate::{channel::tracked_channels::TrackedChannels, errors::AppError};
use chrono::{DateTime, Utc};
use database_connection::get_database_connection;
use entities::*;
use entity_extensions::prelude::*;
use entity_extensions::stream::twitch_stream_response::TwitchStreamData;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::OnConflict;
use sea_orm::DatabaseConnection;
use sea_orm::*;
use std::collections::HashSet;
use std::time::Duration;

const UPDATE_WAIT_TIME: Duration = Duration::new(10, 0);

pub async fn update_channel_live_streams(tracked_channels: TrackedChannels) -> ! {
  let database_connection = get_database_connection().await;
  let mut active_streams = ActiveStreams::default();

  loop {
    let channels = tracked_channels.all_channels();
    let current_live_channels = match stream::Model::get_active_livestreams(channels).await {
      Ok(live_channels) => live_channels,
      Err(error) => {
        tracing::error!("Failed to retrieve current live channels. Reason: {error}");

        tokio::time::sleep(UPDATE_WAIT_TIME).await;

        continue;
      }
    };
    let current_time = Utc::now();

    let update_result = update_streams(
      &tracked_channels,
      &mut active_streams,
      current_live_channels,
      database_connection,
      current_time,
    )
    .await;

    if let Err(error) = update_result {
      tracing::error!("Failed to update livestreams. Reason: {error}");
    }

    tokio::time::sleep(UPDATE_WAIT_TIME).await;
  }
}

async fn update_streams(
  tracked_channels: &TrackedChannels,
  active_streams: &mut ActiveStreams,
  current_live_channels: Vec<TwitchStreamData>,
  database_connection: &DatabaseConnection,
  current_time: DateTime<Utc>,
) -> Result<(), AppError> {
  let current_active_stream_twitch_ids: HashSet<&str> = current_live_channels
    .iter()
    .map(|stream| stream.stream_id.as_str())
    .collect();

  update_finished_streams(
    active_streams,
    current_active_stream_twitch_ids,
    database_connection,
    current_time,
  )
  .await?;

  let livestream_active_models: Vec<stream::ActiveModel> = current_live_channels
    .into_iter()
    .filter_map(|livestream_data| {
      let Ok(stream_twitch_id) = livestream_data.stream_id.parse::<u64>() else {
        tracing::error!(
          "Failed to parse a stream ID. Streamer: {:?}. Value: {:?}",
          livestream_data.user_login,
          livestream_data.stream_id
        );

        return None;
      };
      let Some(channel) = tracked_channels.get_channel(&livestream_data.user_login) else {
        tracing::error!(
          "Failed to find channel when updating active streams. Stream data: {livestream_data:?}"
        );
        return None;
      };

      if active_streams.contains_stream(&livestream_data.stream_id) {
        return None;
      } else {
        active_streams.add_stream(&livestream_data.stream_id);
      }

      Some(stream::ActiveModel {
        twitch_stream_id: Set(stream_twitch_id),
        start_timestamp: Set(Some(livestream_data.started_at)),
        twitch_user_id: Set(channel.id),
        title: Set(Some(livestream_data.title)),
        ..Default::default()
      })
    })
    .collect();

  stream::Entity::insert_many(livestream_active_models)
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

async fn update_finished_streams(
  active_streams: &mut ActiveStreams,
  current_active_stream_twitch_ids: HashSet<&str>,
  database_connection: &DatabaseConnection,
  current_time: DateTime<Utc>,
) -> Result<(), AppError> {
  let ended_stream_ids: Vec<String> = active_streams.extract_if(|stream_twitch_id| {
    !current_active_stream_twitch_ids.contains(stream_twitch_id.as_str())
  });

  stream::Entity::update_many()
    .col_expr(stream::Column::EndTimestamp, Expr::value(current_time))
    .filter(stream::Column::TwitchStreamId.is_in(ended_stream_ids))
    .exec(database_connection)
    .await?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::TimeZone;
  use entities::twitch_user;
  use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};
  use std::collections::HashMap;

  fn create_test_tracked_channels() -> TrackedChannels {
    let mut channels = HashMap::new();
    channels.insert(
      "testuser1".to_string(),
      twitch_user::Model {
        id: 1,
        twitch_id: 12345,
        login_name: "testuser1".to_string(),
        display_name: "TestUser1".to_string(),
      },
    );
    channels.insert(
      "testuser2".to_string(),
      twitch_user::Model {
        id: 2,
        twitch_id: 67890,
        login_name: "testuser2".to_string(),
        display_name: "TestUser2".to_string(),
      },
    );

    TrackedChannels::new_from_map(channels)
  }

  fn create_stream_data(
    stream_id: &str,
    user_login: &str,
    title: &str,
    started_at: DateTime<Utc>,
  ) -> TwitchStreamData {
    TwitchStreamData {
      stream_id: stream_id.to_string(),
      user_id: "123".to_string(),
      user_login: user_login.to_string(),
      stream_status_type: "live".to_string(),
      title: title.to_string(),
      started_at,
    }
  }

  #[tokio::test]
  async fn test_new_stream_insertion() {
    let tracked_channels = create_test_tracked_channels();
    let mut active_streams = ActiveStreams::default();
    let current_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
    let start_time = Utc.with_ymd_and_hms(2024, 1, 1, 11, 0, 0).unwrap();

    let current_live_channels = vec![create_stream_data(
      "999888777",
      "testuser1",
      "Test Stream Title",
      start_time,
    )];

    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_exec_results([
        MockExecResult {
          last_insert_id: 0,
          rows_affected: 0,
        },
        MockExecResult {
          last_insert_id: 1,
          rows_affected: 1,
        },
      ])
      .into_connection();

    let result = update_streams(
      &tracked_channels,
      &mut active_streams,
      current_live_channels,
      &mock_database,
      current_time,
    )
    .await;

    assert!(result.is_ok());
    assert!(active_streams.contains_stream("999888777"));

    let transaction_log = mock_database.into_transaction_log();
    assert_eq!(
      transaction_log.len(),
      2,
      "Expected 2 database operations (UPDATE and INSERT)"
    );
  }

  #[tokio::test]
  async fn test_stream_already_active_on_startup_ignored() {
    let tracked_channels = create_test_tracked_channels();
    let mut active_streams = ActiveStreams::default();
    let current_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
    let start_time = Utc.with_ymd_and_hms(2024, 1, 1, 11, 0, 0).unwrap();

    let current_live_channels = vec![create_stream_data(
      "111222333",
      "testuser1",
      "Existing Stream",
      start_time,
    )];

    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_exec_results([
        MockExecResult {
          last_insert_id: 0,
          rows_affected: 0,
        },
        MockExecResult {
          last_insert_id: 0,
          rows_affected: 0,
        },
      ])
      .into_connection();

    let result = update_streams(
      &tracked_channels,
      &mut active_streams,
      current_live_channels,
      &mock_database,
      current_time,
    )
    .await;

    assert!(result.is_ok());
    assert!(active_streams.contains_stream("111222333"));

    let transaction_log = mock_database.into_transaction_log();
    assert_eq!(transaction_log.len(), 2, "Expected 2 database operations");
  }

  #[tokio::test]
  async fn test_stream_ending_updates_end_timestamp() {
    let tracked_channels = create_test_tracked_channels();
    let mut active_streams = ActiveStreams::default();
    active_streams.add_stream("555666777");

    let current_time = Utc.with_ymd_and_hms(2024, 1, 1, 14, 0, 0).unwrap();
    let current_live_channels = vec![];

    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_exec_results([MockExecResult {
        last_insert_id: 0,
        rows_affected: 1,
      }])
      .into_connection();

    let result = update_streams(
      &tracked_channels,
      &mut active_streams,
      current_live_channels,
      &mock_database,
      current_time,
    )
    .await;

    assert!(result.is_ok());
    assert!(!active_streams.contains_stream("555666777"));

    let transaction_log = mock_database.into_transaction_log();
    assert_eq!(
      transaction_log.len(),
      1,
      "Expected 1 database operation (UPDATE for ended stream, no INSERT since no new streams)"
    );
  }

  #[tokio::test]
  async fn test_multiple_new_streams() {
    let tracked_channels = create_test_tracked_channels();
    let mut active_streams = ActiveStreams::default();
    let current_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
    let start_time = Utc.with_ymd_and_hms(2024, 1, 1, 11, 0, 0).unwrap();

    let current_live_channels = vec![
      create_stream_data("111111111", "testuser1", "Stream 1", start_time),
      create_stream_data("222222222", "testuser2", "Stream 2", start_time),
    ];

    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_exec_results([
        MockExecResult {
          last_insert_id: 0,
          rows_affected: 0,
        },
        MockExecResult {
          last_insert_id: 1,
          rows_affected: 2,
        },
      ])
      .into_connection();

    let result = update_streams(
      &tracked_channels,
      &mut active_streams,
      current_live_channels,
      &mock_database,
      current_time,
    )
    .await;

    assert!(result.is_ok());
    assert!(active_streams.contains_stream("111111111"));
    assert!(active_streams.contains_stream("222222222"));

    let transaction_log = mock_database.into_transaction_log();
    assert_eq!(transaction_log.len(), 2, "Expected 2 database operations");
  }

  #[tokio::test]
  async fn test_mixed_scenario_new_active_and_ended_streams() {
    let tracked_channels = create_test_tracked_channels();
    let mut active_streams = ActiveStreams::default();
    active_streams.add_stream("100000001");
    active_streams.add_stream("100000002");

    let current_time = Utc.with_ymd_and_hms(2024, 1, 1, 15, 0, 0).unwrap();
    let start_time = Utc.with_ymd_and_hms(2024, 1, 1, 14, 0, 0).unwrap();

    let current_live_channels = vec![
      create_stream_data("100000001", "testuser1", "Still Live", start_time),
      create_stream_data("300000001", "testuser2", "New Stream", start_time),
    ];

    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_exec_results([
        MockExecResult {
          last_insert_id: 0,
          rows_affected: 1,
        },
        MockExecResult {
          last_insert_id: 1,
          rows_affected: 1,
        },
      ])
      .into_connection();

    let result = update_streams(
      &tracked_channels,
      &mut active_streams,
      current_live_channels,
      &mock_database,
      current_time,
    )
    .await;

    assert!(result.is_ok());
    assert!(active_streams.contains_stream("100000001"));
    assert!(active_streams.contains_stream("300000001"));
    assert!(!active_streams.contains_stream("100000002"));

    let transaction_log = mock_database.into_transaction_log();
    assert_eq!(
      transaction_log.len(),
      2,
      "Expected 2 database operations (UPDATE for ended stream, INSERT for new stream)"
    );
  }

  #[tokio::test]
  async fn test_stream_with_invalid_id_skipped() {
    let tracked_channels = create_test_tracked_channels();
    let mut active_streams = ActiveStreams::default();
    let current_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
    let start_time = Utc.with_ymd_and_hms(2024, 1, 1, 11, 0, 0).unwrap();

    let mut invalid_stream =
      create_stream_data("invalid_id", "testuser1", "Invalid Stream", start_time);
    invalid_stream.stream_id = "not_a_number".to_string();

    let valid_stream = create_stream_data("123456789", "testuser1", "Valid Stream", start_time);

    let current_live_channels = vec![invalid_stream, valid_stream];

    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_exec_results([
        MockExecResult {
          last_insert_id: 0,
          rows_affected: 0,
        },
        MockExecResult {
          last_insert_id: 1,
          rows_affected: 1,
        },
      ])
      .into_connection();

    let result = update_streams(
      &tracked_channels,
      &mut active_streams,
      current_live_channels,
      &mock_database,
      current_time,
    )
    .await;

    assert!(result.is_ok());
    assert!(active_streams.contains_stream("123456789"));
    assert!(!active_streams.contains_stream("not_a_number"));

    let transaction_log = mock_database.into_transaction_log();
    assert_eq!(
      transaction_log.len(),
      2,
      "Expected 2 database operations (UPDATE, INSERT for valid stream only)"
    );
  }

  #[tokio::test]
  async fn test_stream_for_untracked_channel_skipped() {
    let tracked_channels = create_test_tracked_channels();
    let mut active_streams = ActiveStreams::default();
    let current_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
    let start_time = Utc.with_ymd_and_hms(2024, 1, 1, 11, 0, 0).unwrap();

    let untracked_stream = create_stream_data(
      "987654321",
      "untracked_user",
      "Untracked Stream",
      start_time,
    );
    let tracked_stream = create_stream_data("123456789", "testuser1", "Tracked Stream", start_time);

    let current_live_channels = vec![untracked_stream, tracked_stream];

    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_exec_results([
        MockExecResult {
          last_insert_id: 0,
          rows_affected: 0,
        },
        MockExecResult {
          last_insert_id: 1,
          rows_affected: 1,
        },
      ])
      .into_connection();

    let result = update_streams(
      &tracked_channels,
      &mut active_streams,
      current_live_channels,
      &mock_database,
      current_time,
    )
    .await;

    assert!(result.is_ok());
    assert!(active_streams.contains_stream("123456789"));
    assert!(!active_streams.contains_stream("987654321"));

    let transaction_log = mock_database.into_transaction_log();
    assert_eq!(
      transaction_log.len(),
      2,
      "Expected 2 database operations (UPDATE, INSERT for tracked stream only)"
    );
  }

  #[tokio::test]
  async fn test_no_streams_active() {
    let tracked_channels = create_test_tracked_channels();
    let mut active_streams = ActiveStreams::default();
    let current_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();

    let current_live_channels = vec![];

    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_exec_results([MockExecResult {
        last_insert_id: 0,
        rows_affected: 0,
      }])
      .into_connection();

    let result = update_streams(
      &tracked_channels,
      &mut active_streams,
      current_live_channels,
      &mock_database,
      current_time,
    )
    .await;

    assert!(result.is_ok());

    let transaction_log = mock_database.into_transaction_log();
    assert_eq!(
      transaction_log.len(),
      1,
      "Expected 1 database operation (UPDATE only, no INSERT when no streams)"
    );
  }

  #[tokio::test]
  async fn test_stream_data_preserved_correctly() {
    let tracked_channels = create_test_tracked_channels();
    let mut active_streams = ActiveStreams::default();
    let current_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
    let start_time = Utc.with_ymd_and_hms(2024, 1, 1, 11, 30, 0).unwrap();

    let stream_title = "Amazing Stream with Special Title!";
    let current_live_channels = vec![create_stream_data(
      "555555555",
      "testuser1",
      stream_title,
      start_time,
    )];

    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_exec_results([
        MockExecResult {
          last_insert_id: 0,
          rows_affected: 0,
        },
        MockExecResult {
          last_insert_id: 1,
          rows_affected: 1,
        },
      ])
      .into_connection();

    let result = update_streams(
      &tracked_channels,
      &mut active_streams,
      current_live_channels,
      &mock_database,
      current_time,
    )
    .await;

    assert!(result.is_ok());
    assert!(active_streams.contains_stream("555555555"));

    let transaction_log = mock_database.into_transaction_log();
    assert_eq!(transaction_log.len(), 2, "Expected 2 database operations");
  }
}
