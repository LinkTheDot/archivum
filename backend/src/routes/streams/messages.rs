use crate::app::InterfaceConfig;
use crate::data_transfer_objects::messages_with_user::StreamMessageWithUser;
use crate::data_transfer_objects::stream_message::{StreamMessageDto, StreamMessageUser};
use crate::error::AppError;
use crate::response_models::paginatied_response::*;
use crate::routes::helpers::get_stream::{GetStream, get_stream};
use crate::routes::helpers::serde::deserialize_from_string;
use axum::extract::{Query, State};
use entities::*;
use sea_orm::*;

const MAX_PAGE_SIZE: u64 = 10_000;
const MIN_PAGE_SIZE: u64 = 1;

fn default_page_size() -> u64 {
  MAX_PAGE_SIZE
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

  #[serde(default, deserialize_with = "deserialize_from_string")]
  page: u64,

  #[serde(
    default = "default_page_size",
    deserialize_with = "deserialize_from_string"
  )]
  page_size: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct StreamMessagesResponse {
  pub stream: stream::Model,
  pub messages: Vec<StreamMessageDto>,
}

struct FetchedMessages {
  messages: Vec<(stream_message::Model, Option<StreamMessageUser>)>,
  page: u64,
  number_of_pages: u64,
  number_of_items: u64,
  page_size: u64,
}

#[axum::debug_handler]
pub async fn get_stream_messages(
  Query(query_payload): Query<StreamMessagesQuery>,
  State(interface_config): State<InterfaceConfig>,
) -> Result<axum::Json<PaginatedResponse<StreamMessagesResponse>>, AppError> {
  tracing::info!("Got a stream messages request: {query_payload:?}");

  let database_connection = interface_config.database_connection();
  let stream = get_stream(&query_payload, database_connection).await?;
  let base_query = build_base_query(&stream);

  let fetched_messages = if query_payload.dump {
    all_messages(base_query, database_connection).await?
  } else {
    paginated_messages(&query_payload, base_query, database_connection).await?
  };

  let messages_dtos = StreamMessageDto::convert_messages_with_users(
    fetched_messages.messages,
    database_connection,
    query_payload.no_emotes,
  )
  .await?;

  Ok(axum::Json(PaginatedResponse {
    data: StreamMessagesResponse {
      stream,
      messages: messages_dtos,
    },
    pagination: Pagination {
      total_items: fetched_messages.number_of_items,
      total_pages: fetched_messages.number_of_pages,
      page: fetched_messages.page,
      page_size: fetched_messages.page_size,
    },
  }))
}

fn build_base_query(stream: &stream::Model) -> Selector<SelectModel<StreamMessageWithUser>> {
  stream_message::Entity::find()
    .column_as(
      twitch_user::Column::TwitchId,
      StreamMessageWithUser::USER_TWITCH_ID_COLUMN,
    )
    .column_as(
      twitch_user::Column::LoginName,
      StreamMessageWithUser::USER_LOGIN_NAME_COLUMN,
    )
    .column_as(
      twitch_user::Column::DisplayName,
      StreamMessageWithUser::USER_DISPLAY_NAME_COLUMN,
    )
    .join(
      JoinType::LeftJoin,
      stream_message::Relation::TwitchUser1.def(),
    )
    .filter(stream_message::Column::StreamId.eq(stream.id))
    .order_by_asc(stream_message::Column::Timestamp)
    .into_model::<StreamMessageWithUser>()
}

async fn all_messages(
  base_query: Selector<SelectModel<StreamMessageWithUser>>,
  database_connection: &DatabaseConnection,
) -> Result<FetchedMessages, AppError> {
  let rows = base_query.all(database_connection).await?;
  let messages: Vec<(stream_message::Model, Option<StreamMessageUser>)> = rows
    .into_iter()
    .map(StreamMessageWithUser::into_pair)
    .collect();
  let total = messages.len();

  Ok(FetchedMessages {
    messages,
    page: 0,
    number_of_pages: 1,
    number_of_items: total as u64,
    page_size: total as u64,
  })
}

async fn paginated_messages(
  query_payload: &StreamMessagesQuery,
  base_query: Selector<SelectModel<StreamMessageWithUser>>,
  database_connection: &DatabaseConnection,
) -> Result<FetchedMessages, AppError> {
  let page_size = query_payload.page_size.clamp(MIN_PAGE_SIZE, MAX_PAGE_SIZE);
  let page = query_payload.page;
  let paginated = base_query.paginate(database_connection, page_size);
  let rows = paginated.fetch_page(page).await?;
  let messages: Vec<(stream_message::Model, Option<StreamMessageUser>)> = rows
    .into_iter()
    .map(StreamMessageWithUser::into_pair)
    .collect();
  let ItemsAndPagesNumber {
    number_of_items,
    number_of_pages,
  } = paginated.num_items_and_pages().await?;

  Ok(FetchedMessages {
    messages,
    page,
    number_of_pages,
    number_of_items,
    page_size,
  })
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
