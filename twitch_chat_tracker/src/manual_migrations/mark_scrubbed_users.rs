//! 2026-17-02
//!
//! This migration gets all users with messages before February 2025 and marks them
//! as having had messages scraped from Spanix.
//!
//! This date was the start of this app's tracking and had messages filled from the scraper
//! before this date. The point of the migration is to add the new table indicating these
//! users were scraped already.

/// 2025 February 1st.
///
/// Any day before this date should be indicated as scrubbed data.
const SCRUBBED_END_DATE: &str = "2025-02-01 00:00:00";
const INSERT_BATCH_SIZE: usize = 1_000;

use database_connection::get_database_connection;
use entities::*;
use sea_orm::{prelude::Expr, *};
use twitch_chat_tracker::errors::AppError;

#[allow(dead_code)]
pub async fn mark_scrubbed_users_for_channel(channel_id: i32) -> ! {
  if let Err(error) = run(channel_id).await {
    tracing::error!("Failed to parse stream names from file. Reason: `{error}`");

    std::process::exit(1)
  }

  std::process::exit(0)
}

async fn run(channel_id: i32) -> Result<(), AppError> {
  let database_connection = get_database_connection().await;
  let users = get_users(channel_id, database_connection).await?;
  let user_count = users.len();

  let total_batches = user_count / INSERT_BATCH_SIZE;

  for (batch_index, chunk) in users.chunks(INSERT_BATCH_SIZE).enumerate() {
    println!("Processing batch {}/{total_batches}", batch_index + 1);

    let models: Vec<scrubbed_user_messages::ActiveModel> = chunk
      .iter()
      .map(|user| scrubbed_user_messages::ActiveModel {
        channel_id: Set(channel_id),
        twitch_user_id: Set(user.id),
        ..Default::default()
      })
      .collect();

    let result = scrubbed_user_messages::Entity::insert_many(models)
      .exec(database_connection)
      .await;

    if let Err(error) = result {
      tracing::error!("Failed to batch insert scrubbed_user_messages. Reason: {error}");
    }
  }

  Ok(())
}

/// Retrieves all of the users with messages before the SCRUBBED_END_DATE.
//
// SELECT
//   twitch_user.id,
//   twitch_user.twitch_id,
//   twitch_user.display_name,
//   twitch_user.login_name
// FROM twitch_user
// INNER JOIN stream_message
//   ON twitch_user.id = stream_message.twitch_user_id
//   AND stream_message.channel_id = 1
// WHERE stream_message.timestamp < '2025-02-01 00:00:00'
async fn get_users(
  channel_id: i32,
  database_connection: &DatabaseConnection,
) -> Result<Vec<twitch_user::Model>, AppError> {
  twitch_user::Entity::find()
    .distinct()
    .join(
      JoinType::InnerJoin,
      stream_message::Relation::TwitchUser1
        .def()
        .rev()
        .on_condition(move |_left, right| {
          Condition::all().add(Expr::col((right, stream_message::Column::ChannelId)).eq(channel_id))
        }),
    )
    .filter(stream_message::Column::Timestamp.lt(SCRUBBED_END_DATE))
    .all(database_connection)
    .await
    .map_err(Into::into)
}
