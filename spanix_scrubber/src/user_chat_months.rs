use crate::response_models::avaiable_logs::LogEntry;
use entities::*;
use sea_orm::prelude::Expr;
use sea_orm::*;
use std::collections::{HashMap, HashSet};
use twitch_chat_tracker::errors::AppError;

const YEAR_COLUMN_NAME: &str = "chat_year";
const MONTH_COLUMN_NAME: &str = "chat_month";

#[derive(Debug, FromQueryResult)]
struct UserChatMonthRow {
  twitch_user_id: i32,
  chat_year: i32,
  chat_month: i32,
}

#[derive(Debug, Default)]
pub struct UserChatMonths {
  months_by_user: HashMap<i32, HashSet<LogEntry>>,
}

impl UserChatMonths {
  /// Retrieves every distinct (user, year-month) pair from stream messages
  /// for the given channel in a single query.
  ///
  /// Returns a map of user IDs to the months they have chatted in,
  /// represented as first-of-month dates.
  pub async fn retrieve_for_channel(
    channel: &twitch_user::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Self, AppError> {
    let timestamp_column = stream_message::Column::Timestamp.to_string();
    let year_column = Expr::cust(format!("YEAR({timestamp_column})"));
    let month_column = Expr::cust(format!("MONTH({timestamp_column})"));

    let rows = stream_message::Entity::find()
      .select_only()
      .column(stream_message::Column::TwitchUserId)
      .column_as(year_column.clone(), YEAR_COLUMN_NAME)
      .column_as(month_column.clone(), MONTH_COLUMN_NAME)
      .filter(stream_message::Column::ChannelId.eq(channel.id))
      .group_by(stream_message::Column::TwitchUserId)
      .group_by(year_column)
      .group_by(month_column)
      .into_model::<UserChatMonthRow>()
      .all(database_connection)
      .await?;

    let mut months_by_user: HashMap<i32, HashSet<LogEntry>> = HashMap::new();

    for row in rows {
      let log_entry = LogEntry {
        year: row.chat_year.to_string(),
        month: row.chat_month.to_string(),
      };

      months_by_user
        .entry(row.twitch_user_id)
        .or_default()
        .insert(log_entry);
    }

    Ok(Self { months_by_user })
  }

  pub fn get(&self, user: &twitch_user::Model) -> Option<&HashSet<LogEntry>> {
    self.months_by_user.get(&user.id)
  }
}
