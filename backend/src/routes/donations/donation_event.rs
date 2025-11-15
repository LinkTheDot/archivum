use crate::data_transfer_objects::donation_event::DonationEventDto;
use crate::error::AppError;
use crate::response_models::paginated_parameters::PaginationParameters;
use crate::response_models::paginatied_response::{PaginatedResponse, Pagination};
use crate::routes::helpers::get_channel::get_channel;
use crate::{app::InterfaceConfig, routes::helpers::get_users::GetUsers};
use axum::extract::{Path, Query, State};
use entities::*;
use sea_orm::*;

const MAX_PAGE_SIZE: u64 = 100;
const MIN_PAGE_SIZE: u64 = 1;

#[derive(Debug, serde::Deserialize)]
pub struct DonationEventQuery {
  maybe_login: Option<String>,
  user_id: Option<String>,

  gift_sub_recipients: Option<bool>,

  #[serde(flatten)]
  pagination_parameters: PaginationParameters,
}

#[axum::debug_handler]
pub async fn get_donations(
  Query(query_payload): Query<DonationEventQuery>,
  State(interface_config): State<InterfaceConfig>,
  channel_login: Option<Path<String>>,
) -> Result<axum::Json<PaginatedResponse<Vec<DonationEventDto>>>, AppError> {
  tracing::info!("Got a donation event request for {channel_login:?}: {query_payload:?}");
  let database_connection = interface_config.database_connection();
  let include_gift_sub_recipients = query_payload.gift_sub_recipients.unwrap_or(false);
  let pagination = query_payload
    .pagination_parameters
    .clamped_page_size(MIN_PAGE_SIZE, MAX_PAGE_SIZE);
  let maybe_user = get_user(&query_payload, database_connection).await?;

  let donations_query = get_donation_query(maybe_user, channel_login, database_connection).await?;

  let paginated_donation_events =
    donations_query.paginate(database_connection, pagination.page_size);
  let donation_events = paginated_donation_events
    .fetch_page(pagination.page)
    .await?;
  let ItemsAndPagesNumber {
    number_of_items,
    number_of_pages,
  } = paginated_donation_events.num_items_and_pages().await?;

  let donation_events = DonationEventDto::from_donation_event_list(
    donation_events,
    include_gift_sub_recipients,
    database_connection,
  )
  .await?;

  Ok(axum::Json(PaginatedResponse {
    data: donation_events,
    pagination: Pagination {
      total_items: number_of_items,
      total_pages: number_of_pages,
      page: pagination.page,
      page_size: pagination.page_size,
    },
  }))
}

async fn get_user(
  query_payload: &DonationEventQuery,
  database_connection: &DatabaseConnection,
) -> Result<Option<twitch_user::Model>, AppError> {
  let Some(user_query) = query_payload.get_maybe_user_query() else {
    return Ok(None);
  };

  user_query
    .one(database_connection)
    .await
    .map_err(Into::into)
}

async fn get_donation_query(
  user: Option<twitch_user::Model>,
  channel_login: Option<Path<String>>,
  database_connection: &DatabaseConnection,
) -> Result<Select<donation_event::Entity>, AppError> {
  let mut donations_query = donation_event::Entity::find();

  if let Some(user) = user {
    donations_query =
      donations_query.filter(donation_event::Column::DonatorTwitchUserId.eq(user.id));
  }

  if let Some(Path(channel_login)) = channel_login {
    let channel = get_channel(channel_login, database_connection).await?;

    donations_query =
      donations_query.filter(donation_event::Column::DonationReceiverTwitchUserId.eq(channel.id));
  }

  donations_query = donations_query.order_by(donation_event::Column::Timestamp, Order::Desc);

  Ok(donations_query)
}

impl GetUsers for DonationEventQuery {
  fn get_login(&self) -> Option<&str> {
    self.maybe_login.as_deref()
  }

  fn get_twitch_id(&self) -> Option<&str> {
    self.user_id.as_deref()
  }
}
