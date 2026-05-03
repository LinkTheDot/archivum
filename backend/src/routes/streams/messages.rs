use crate::app::InterfaceConfig;
use crate::data_transfer_objects::stream_message::StreamMessageDto;
use crate::error::AppError;
use crate::response_models::paginatied_response::*;
use crate::routes::helpers::get_stream::{get_stream, GetStream};
use crate::routes::helpers::serde::deserialize_from_string;
use axum::extract::{Query, State};
use entities::*;
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

  let messages_query = stream_message::Entity::find()
    .filter(stream_message::Column::StreamId.eq(stream.id))
    .order_by_asc(stream_message::Column::Timestamp);

  if query_payload.dump {
    let messages = messages_query.all(database_connection).await?;
    let total = messages.len() as u64;
    let messages_dtos =
      StreamMessageDto::convert_messages(messages, database_connection, query_payload.no_emotes)
        .await?;

    return Ok(axum::Json(PaginatedResponse {
      data: StreamMessagesResponse { stream, messages: messages_dtos },
      pagination: Pagination { total_items: total, total_pages: 1, page: 0, page_size: total },
    }));
  }

  let page_size = query_payload.page_size.clamp(MIN_PAGE_SIZE, MAX_PAGE_SIZE);
  let page = query_payload.page;
  let paginated = messages_query.paginate(database_connection, page_size);
  let messages = paginated.fetch_page(page).await?;

  let messages_dtos =
    StreamMessageDto::convert_messages(messages, database_connection, query_payload.no_emotes)
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
