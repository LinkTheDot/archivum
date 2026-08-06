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

  #[serde(default, deserialize_with = "deserialize_from_option_string")]
  stream_id: Option<u64>,

  #[serde(flatten)]
  pagination_parameters: PaginationParameters,
}

#[derive(Debug, serde::Serialize)]
pub struct UserMessageResponse {
  user: twitch_user::Model,
  channel: twitch_user::Model,

  messages: Vec<StreamMessageDto>,
}

#[axum::debug_handler]
pub async fn get_messages(
  Query(query_payload): Query<UserMessagesQuery>,
  State(interface_config): State<InterfaceConfig>,
  Path(channel_name): Path<String>,
) -> Result<axum::Json<PaginatedResponse<UserMessageResponse>>, AppError> {
  tracing::info!("Got a user messages request: {query_payload:?} For channel: {channel_name:?}");

  let database_connection = interface_config.database_connection();
  let pagination = query_payload
    .pagination_parameters
    .clamped_page_size(MIN_PAGE_SIZE, MAX_PAGE_SIZE);

  let Some(user) = query_payload
    .get_user_query()?
    .one(database_connection)
    .await?
  else {
    return Err(query_payload.get_missing_user_error());
  };
  let channel = get_channel(channel_name, database_connection).await?;

  let user_messages_query = get_user_messages_query(&query_payload, &user, &channel);

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
    },
    pagination: Pagination {
      total_items: number_of_items,
      total_pages: number_of_pages,
      page: pagination.page,
      page_size: pagination.page_size,
    },
  }))
}

fn get_user_messages_query(
  query_payload: &UserMessagesQuery,
  user: &twitch_user::Model,
  channel: &twitch_user::Model,
) -> Select<stream_message::Entity> {
  let mut message_query = stream_message::Entity::find()
    .filter(stream_message::Column::TwitchUserId.eq(user.id))
    .filter(stream_message::Column::ChannelId.eq(channel.id))
    .order_by(stream_message::Column::Timestamp, Order::Desc);

  if let Some(message_search) = &query_payload.message_search {
    message_query = message_query.filter(stream_message::Column::Contents.contains(message_search));
  }

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

  if let Some(stream_id) = query_payload.stream_id {
    message_query = message_query.filter(stream_message::Column::TwitchUserId.eq(stream_id));
  }

  message_query
}

impl GetUsers for UserMessagesQuery {
  fn get_login(&self) -> Option<&str> {
    self.maybe_login.as_deref()
  }

  fn get_twitch_id(&self) -> Option<&str> {
    self.user_id.as_deref()
  }
}
