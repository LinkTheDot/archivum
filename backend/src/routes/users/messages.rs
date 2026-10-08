use crate::app::InterfaceConfig;
use crate::data_transfer_objects::stream_message::StreamMessageDto;
use crate::error::*;
use crate::response_models::{paginated_parameters::*, paginatied_response::*};
use crate::routes::helpers::get_channel::get_channel;
use crate::routes::helpers::get_users::GetUsers;
use crate::routes::helpers::serde::*;
use axum::extract::{Path, Query, State};
use chrono::Utc;
use entities::*;
use entity_extensions::stream::StreamExtensions;
use sea_orm::sea_query::Expr;
use sea_orm::*;

const MAX_PAGE_SIZE: u64 = 1_000;
const MIN_PAGE_SIZE: u64 = 1;

#[derive(Debug, serde::Deserialize)]
pub struct UserMessagesQuery {
  maybe_login: Option<String>,
  user_id: Option<String>,

  message_search: Option<String>,

  #[serde(default, deserialize_with = "deserialize_from_option_string")]
  date_start: Option<chrono::DateTime<Utc>>,
  #[serde(default, deserialize_with = "deserialize_from_option_string")]
  date_end: Option<chrono::DateTime<Utc>>,

  /// Based on Twitch's stream id
  #[serde(default, deserialize_with = "deserialize_from_option_string")]
  stream_id: Option<u64>,

  /// Switches to the "paginate by month" mode: lifts the page size cap and populates
  /// `available_months` on the response so the front-end can offer a month picker.
  #[serde(default)]
  per_month: bool,

  #[serde(flatten)]
  pagination_parameters: PaginationParameters,
}

#[derive(Debug, serde::Serialize)]
pub struct UserMessageResponse {
  user: twitch_user::Model,
  channel: twitch_user::Model,

  messages: Vec<StreamMessageDto>,

  /// The distinct `YYYY-MM` months this user has messages for, ignoring `date_start`/
  /// `date_end` so the full range stays selectable. Only populated when `per_month` is set.
  available_months: Vec<String>,
}

#[axum::debug_handler]
pub async fn get_messages(
  Query(query_payload): Query<UserMessagesQuery>,
  State(interface_config): State<InterfaceConfig>,
  Path(channel_name): Path<String>,
) -> Result<axum::Json<PaginatedResponse<UserMessageResponse>>, AppError> {
  tracing::info!("Got a user messages request: {query_payload:?} For channel: {channel_name:?}");

  let database_connection = interface_config.database_connection();
  let max_page_size = if query_payload.per_month {
    u64::MAX
  } else {
    MAX_PAGE_SIZE
  };
  let pagination = query_payload
    .pagination_parameters
    .clamped_page_size(MIN_PAGE_SIZE, max_page_size);

  let Some(user) = query_payload
    .get_user_query()?
    .one(database_connection)
    .await?
  else {
    return Err(query_payload.get_missing_user_error());
  };
  let channel = get_channel(channel_name, database_connection).await?;

  let stream = if let Some(twitch_stream_id) = query_payload.stream_id {
    Some(
      stream::Model::from_twitch_id(twitch_stream_id, database_connection)
        .await?
        .ok_or(AppError::FailedToFindStreamByTwitchStreamId { twitch_stream_id })?,
    )
  } else {
    None
  };

  let scope_query = get_message_scope_query(&query_payload, &user, &channel, stream.as_ref());

  let available_months = if query_payload.per_month {
    get_available_months(scope_query.clone(), database_connection).await?
  } else {
    Vec::new()
  };

  let user_messages_query = apply_date_filters(scope_query, &query_payload)
    .order_by(stream_message::Column::Timestamp, Order::Desc);

  let paginated_user_messages =
    user_messages_query.paginate(database_connection, pagination.page_size);
  let user_messages = paginated_user_messages.fetch_page(pagination.page).await?;

  let user_message_count = user_messages.len();
  let user_messages_dtos =
    StreamMessageDto::convert_messages(user_messages, database_connection, false).await?;

  if user_message_count != user_messages_dtos.len() {
    tracing::warn!("Mismatch in user message count after DTO conversion.");
  }

  let ItemsAndPagesNumber {
    number_of_items,
    number_of_pages,
  } = paginated_user_messages.num_items_and_pages().await?;

  Ok(axum::Json(PaginatedResponse {
    data: UserMessageResponse {
      user,
      channel,
      messages: user_messages_dtos,
      available_months,
    },
    pagination: Pagination {
      total_items: number_of_items,
      total_pages: number_of_pages,
      page: pagination.page,
      page_size: pagination.page_size,
    },
  }))
}

/// Filters that scope which messages we're looking at (who, where, and which stream/
/// search text) but say nothing about *when* — used both for the paginated message
/// list (with date filters layered on top) and for the `available_months` listing
/// (which needs to see every month regardless of the currently selected date range).
fn get_message_scope_query(
  query_payload: &UserMessagesQuery,
  user: &twitch_user::Model,
  channel: &twitch_user::Model,
  stream: Option<&stream::Model>,
) -> Select<stream_message::Entity> {
  let mut message_query = stream_message::Entity::find()
    .filter(stream_message::Column::TwitchUserId.eq(user.id))
    .filter(stream_message::Column::ChannelId.eq(channel.id));

  if let Some(message_search) = &query_payload.message_search {
    message_query = message_query.filter(stream_message::Column::Contents.contains(message_search));
  }

  if let Some(stream) = stream {
    message_query = message_query.filter(stream_message::Column::StreamId.eq(stream.id));
  }

  message_query
}

fn apply_date_filters(
  mut message_query: Select<stream_message::Entity>,
  query_payload: &UserMessagesQuery,
) -> Select<stream_message::Entity> {
  if let (Some(date_start), Some(date_end)) = (query_payload.date_start, query_payload.date_end) {
    message_query =
      message_query.filter(stream_message::Column::Timestamp.between(date_start, date_end));
  } else {
    if let Some(date_start) = query_payload.date_start {
      message_query = message_query.filter(stream_message::Column::Timestamp.gte(date_start));
    }

    if let Some(date_end) = query_payload.date_end {
      message_query = message_query.filter(stream_message::Column::Timestamp.lte(date_end));
    }
  }

  message_query
}

#[derive(Debug, FromQueryResult)]
struct AvailableMonthRow {
  month: String,
}

/// Returns the distinct `YYYY-MM` months present in `scope_query`, newest first.
async fn get_available_months(
  scope_query: Select<stream_message::Entity>,
  database_connection: &DatabaseConnection,
) -> Result<Vec<String>, AppError> {
  let month_expr = Expr::cust_with_expr(
    "DATE_FORMAT(?, '%Y-%m')",
    Expr::col(stream_message::Column::Timestamp),
  );

  let rows = scope_query
    .select_only()
    .column_as(month_expr, "month")
    .distinct()
    .into_model::<AvailableMonthRow>()
    .all(database_connection)
    .await?;

  let mut months: Vec<String> = rows.into_iter().map(|row| row.month).collect();
  months.sort_unstable_by(|a, b| b.cmp(a));

  Ok(months)
}

impl GetUsers for UserMessagesQuery {
  fn get_login(&self) -> Option<&str> {
    self.maybe_login.as_deref()
  }

  fn get_twitch_id(&self) -> Option<&str> {
    self.user_id.as_deref()
  }
}
