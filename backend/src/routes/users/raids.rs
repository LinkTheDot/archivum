use crate::{
  app::InterfaceConfig,
  data_transfer_objects::raid::RaidDto,
  error::AppError,
  response_models::{
    paginated_parameters::PaginationParameters,
    paginatied_response::{PaginatedResponse, Pagination},
  },
  routes::helpers::get_users::GetUsers,
};
use axum::extract::{Query, State};
use entities::{raid, twitch_user};
use sea_orm::*;

const MAX_PAGE_SIZE: u64 = 100;
const MIN_PAGE_SIZE: u64 = 1;

#[derive(Debug, serde::Deserialize)]
pub struct RaidQuery {
  channel_login: Option<String>,
  channel_id: Option<String>,

  #[serde(flatten)]
  raider: RaiderInfo,

  #[serde(flatten)]
  pagination_parameters: PaginationParameters,
}

#[derive(Debug, serde::Deserialize)]
pub struct RaiderInfo {
  raider_login: Option<String>,
  raider_id: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct RaidResponse {
  channel: twitch_user::Model,
  raids: Vec<RaidDto>,
}

#[axum::debug_handler]
pub async fn get_raids(
  Query(query_payload): Query<RaidQuery>,
  State(interface_config): State<InterfaceConfig>,
) -> Result<axum::Json<PaginatedResponse<RaidResponse>>, AppError> {
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
  let raid_query = get_raids_query(&user, &query_payload, database_connection).await?;
  let paginate_raids = raid_query.paginate(database_connection, pagination.page_size);

  let fetched_raids = paginate_raids.fetch_page(pagination.page).await?;
  let ItemsAndPagesNumber {
    number_of_items,
    number_of_pages,
  } = paginate_raids.num_items_and_pages().await?;

  let raid_dtos = RaidDto::from_raid_list(fetched_raids, database_connection).await?;
  let raid_response = RaidResponse {
    channel: user,
    raids: raid_dtos,
  };

  Ok(axum::Json(PaginatedResponse {
    data: raid_response,
    pagination: Pagination {
      total_items: number_of_items,
      total_pages: number_of_pages,
      page: pagination.page,
      page_size: pagination.page_size,
    },
  }))
}

async fn get_raids_query(
  user: &twitch_user::Model,
  query_payload: &RaidQuery,
  database_connection: &DatabaseConnection,
) -> Result<Select<raid::Entity>, AppError> {
  let mut raid_query = raid::Entity::find().filter(raid::Column::TwitchUserId.eq(user.id));

  let maybe_raider = query_payload.raider.get_maybe_user_query();

  if let Some(raider_query) = maybe_raider {
    let Some(raider) = raider_query.one(database_connection).await? else {
      return Err(query_payload.raider.get_missing_user_error());
    };

    raid_query = raid_query.filter(raid::Column::RaiderTwitchUserId.eq(raider.id));
  }

  raid_query = raid_query.order_by(raid::Column::Timestamp, Order::Desc);

  Ok(raid_query)
}

impl GetUsers for RaidQuery {
  fn get_login(&self) -> Option<&str> {
    self.channel_login.as_deref()
  }

  fn get_twitch_id(&self) -> Option<&str> {
    self.channel_id.as_deref()
  }
}

impl GetUsers for RaiderInfo {
  fn get_login(&self) -> Option<&str> {
    self.raider_login.as_deref()
  }

  fn get_twitch_id(&self) -> Option<&str> {
    self.raider_id.as_deref()
  }
}
