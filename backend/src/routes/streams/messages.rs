use crate::app::InterfaceConfig;
use crate::data_transfer_objects::stream_message::{StreamMessageDto, StreamMessageUser};
use crate::error::AppError;
use crate::response_models::paginatied_response::*;
use crate::routes::helpers::get_stream::{get_stream, GetStream};
use crate::routes::helpers::serde::deserialize_from_string;
use axum::extract::{Query, State};
use entities::*;
use sea_orm::prelude::DateTimeUtc;
use sea_orm::*;

const MAX_PAGE_SIZE: u64 = 10_000;
const MIN_PAGE_SIZE: u64 = 1;

fn default_page_size() -> u64 {
  MAX_PAGE_SIZE
}

fn default_page() -> u64 {
  0
}

#[derive(Debug, serde::Deserialize)]
pub struct StreamMessagesQuery {
  stream_id: Option<String>,
  twitch_stream_id: Option<String>,
  twitch_vod_id: Option<String>,

  #[serde(default)]
  no_emotes: bool,

  #[serde(default)]
  dump: bool,

  #[serde(default = "default_page", deserialize_with = "deserialize_from_string")]
  page: u64,

  #[serde(default = "default_page_size", deserialize_with = "deserialize_from_string")]
  page_size: u64,
}

#[derive(sea_orm::FromQueryResult)]
struct StreamMessageWithUser {
  id: i32,
  is_first_message: i8,
  timestamp: DateTimeUtc,
  emote_only: i8,
  contents: Option<String>,
  twitch_user_id: i32,
  channel_id: i32,
  stream_id: Option<i32>,
  is_subscriber: i8,
  origin_id: Option<String>,
  is_from_subscription_message: i8,
  user_twitch_id: Option<i32>,
  user_login_name: Option<String>,
  user_display_name: Option<String>,
}

impl StreamMessageWithUser {
  fn into_pair(self) -> (stream_message::Model, Option<StreamMessageUser>) {
    let user = match (self.user_twitch_id, self.user_login_name, self.user_display_name) {
      (Some(twitch_id), Some(login_name), Some(display_name)) => {
        Some(StreamMessageUser { twitch_id, login_name, display_name })
      }
      _ => None,
    };
    let message = stream_message::Model {
      id: self.id,
      is_first_message: self.is_first_message,
      timestamp: self.timestamp,
      emote_only: self.emote_only,
      contents: self.contents,
      twitch_user_id: self.twitch_user_id,
      channel_id: self.channel_id,
      stream_id: self.stream_id,
      is_subscriber: self.is_subscriber,
      origin_id: self.origin_id,
      is_from_subscription_message: self.is_from_subscription_message,
    };
    (message, user)
  }
}

#[derive(Debug, serde::Serialize)]
pub struct StreamMessagesResponse {
  pub stream: stream::Model,
  pub messages: Vec<StreamMessageDto>,
}

#[axum::debug_handler]
pub async fn get_stream_messages(
  Query(query_payload): Query<StreamMessagesQuery>,
  State(interface_config): State<InterfaceConfig>,
) -> Result<axum::Json<PaginatedResponse<StreamMessagesResponse>>, AppError> {
  tracing::info!("Got a stream messages request: {query_payload:?}");

  let database_connection = interface_config.database_connection();

  let stream = get_stream(&query_payload, database_connection).await?;

  let base_query = stream_message::Entity::find()
    .column_as(twitch_user::Column::TwitchId, "user_twitch_id")
    .column_as(twitch_user::Column::LoginName, "user_login_name")
    .column_as(twitch_user::Column::DisplayName, "user_display_name")
    .join(JoinType::LeftJoin, stream_message::Relation::TwitchUser1.def())
    .filter(stream_message::Column::StreamId.eq(stream.id))
    .order_by_asc(stream_message::Column::Timestamp)
    .into_model::<StreamMessageWithUser>();

  if query_payload.dump {
    let rows = base_query.all(database_connection).await?;
    let total = rows.len() as u64;
    let pairs = rows.into_iter().map(StreamMessageWithUser::into_pair).collect();
    let messages_dtos =
      StreamMessageDto::convert_messages_with_users(pairs, database_connection, query_payload.no_emotes)
        .await?;

    return Ok(axum::Json(PaginatedResponse {
      data: StreamMessagesResponse { stream, messages: messages_dtos },
      pagination: Pagination { total_items: total, total_pages: 1, page: 0, page_size: total },
    }));
  }

  let page_size = query_payload.page_size.clamp(MIN_PAGE_SIZE, MAX_PAGE_SIZE);
  let page = query_payload.page;
  let paginated = base_query.paginate(database_connection, page_size);
  let rows = paginated.fetch_page(page).await?;
  let pairs = rows.into_iter().map(StreamMessageWithUser::into_pair).collect();

  let messages_dtos =
    StreamMessageDto::convert_messages_with_users(pairs, database_connection, query_payload.no_emotes)
      .await?;

  let ItemsAndPagesNumber { number_of_items, number_of_pages } =
    paginated.num_items_and_pages().await?;

  Ok(axum::Json(PaginatedResponse {
    data: StreamMessagesResponse { stream, messages: messages_dtos },
    pagination: Pagination {
      total_items: number_of_items,
      total_pages: number_of_pages,
      page,
      page_size,
    },
  }))
}

impl GetStream for StreamMessagesQuery {
  fn get_stream_id(&self) -> Option<&str> {
    self.stream_id.as_deref()
  }

  fn get_twitch_stream_id(&self) -> Option<&str> {
    self.twitch_stream_id.as_deref()
  }

  fn get_twitch_vod_id(&self) -> Option<&str> {
    self.twitch_vod_id.as_deref()
  }
}
