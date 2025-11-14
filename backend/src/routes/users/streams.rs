use crate::data_transfer_objects::stream::{StreamDto, StreamResponse};
use crate::response_models::paginated_parameters::PaginationParameters;
use crate::response_models::paginatied_response::{PaginatedResponse, Pagination};
use crate::routes::helpers::get_users::GetUsers;
use crate::{app::InterfaceConfig, error::*};
use axum::extract::{Query, State};
use entities::*;
use sea_orm::*;

const MAX_PAGE_SIZE: u64 = 100;
const MIN_PAGE_SIZE: u64 = 1;

#[derive(Debug, serde::Deserialize)]
pub struct StreamQuery {
  maybe_login: Option<String>,
  user_id: Option<String>,

  #[serde(flatten)]
  pagination_parameters: PaginationParameters,
}

#[axum::debug_handler]
pub async fn get_streams(
  Query(query_payload): Query<StreamQuery>,
  State(interface_config): State<InterfaceConfig>,
) -> Result<axum::Json<PaginatedResponse<StreamResponse>>, AppError> {
  tracing::info!("Got a stream request: {query_payload:?}");

  let database_connection = interface_config.database_connection();
  let pagination = query_payload
    .pagination_parameters
    .clamped_page_size(MIN_PAGE_SIZE, MAX_PAGE_SIZE);

  let user_query = query_payload.get_user_query()?;
  let Some(user) = user_query.one(database_connection).await? else {
    return Err(query_payload.get_missing_user_error());
  };
  let stream_query = stream::Entity::find().filter(stream::Column::TwitchUserId.eq(user.id));
  let paginated_streams = stream_query.paginate(database_connection, pagination.page_size);

  let fetched_paginated_streams = paginated_streams.fetch_page(pagination.page).await?;
  let ItemsAndPagesNumber {
    number_of_items,
    number_of_pages,
  } = paginated_streams.num_items_and_pages().await?;

  let mut stream_response =
    StreamDto::response_from_stream_list(&user, fetched_paginated_streams, database_connection)
      .await?;
  stream_response.streams.sort_by(|lhs, rhs| rhs.id.cmp(&lhs.id));

  Ok(axum::Json(PaginatedResponse {
    data: stream_response,
    pagination: Pagination {
      total_items: number_of_items,
      total_pages: number_of_pages,
      page: pagination.page,
      page_size: pagination.page_size,
    },
  }))
}

impl GetUsers for StreamQuery {
  fn get_login(&self) -> Option<&str> {
    self.maybe_login.as_deref()
  }

  fn get_twitch_id(&self) -> Option<&str> {
    self.user_id.as_deref()
  }
}
