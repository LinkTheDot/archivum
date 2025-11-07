use crate::{app::InterfaceConfig, error::AppError};
use axum::extract::State;
use entities::{donation_event, sea_orm_active_enums::EventType, subscription_event};
use sea_orm::*;
use sqlx::types::chrono::{TimeZone, Utc};

const POINTS_PER_BIT: f64 = 0.01;
const POINTS_PER_TIER_1_SUB: f64 = 5.0;
const POINTS_PER_TIER_2_SUB: f64 = 10.0;
const POINTS_PER_TIER_3_SUB: f64 = 25.0;
const POINTS_PER_DOLLAR: f64 = 1.0;

#[derive(Debug, Default, serde::Serialize)]
pub struct SubathonResponse {
  total_points: f64,

  prime_subs: i32,
  tier_1_subs: i32,
  tier_2_subs: i32,
  tier_3_subs: i32,

  bits: i32,
  direct_donations: f64,
}

#[axum::debug_handler]
pub async fn get_subathon_data(
  State(interface_config): State<InterfaceConfig>,
) -> Result<axum::Json<SubathonResponse>, AppError> {
  tracing::info!("Got a subathon request.");

  let database_connection = interface_config.database_connection();
  let chrono::offset::LocalResult::Single(subathon_start_result) =
    Utc.with_ymd_and_hms(2025, 10, 15, 0, 0, 0)
  else {
    return Err(AppError::ChronoError(
      "Failed to build datetime for subathon start.".into(),
    ));
  };
  let mut all_donations = donation_event::Entity::find()
    .filter(donation_event::Column::Timestamp.gte(subathon_start_result))
    .filter(donation_event::Column::DonationReceiverTwitchUserId.eq(1))
    .select_only()
    .column(donation_event::Column::EventType)
    .column(donation_event::Column::SubscriptionTier)
    .column_as(donation_event::Column::Amount.sum(), "sum_amount")
    .group_by(donation_event::Column::EventType)
    .group_by(donation_event::Column::SubscriptionTier)
    .into_model::<DonationSum>()
    .all(database_connection)
    .await?;
  let subscriptions = subscription_event::Entity::find()
    .filter(subscription_event::Column::Timestamp.gte(subathon_start_result))
    .filter(subscription_event::Column::ChannelId.eq(1))
    .select_only()
    .column(subscription_event::Column::SubscriptionTier)
    .into_model::<StrippedSubscriptionEvent>()
    .all(database_connection)
    .await?;
  let subscriptions = subscriptions
    .into_iter()
    .map(Into::into)
    .collect::<Vec<DonationSum>>();
  all_donations.extend(subscriptions);

  let mut subathon_response = SubathonResponse::default();

  for DonationSum {
    event_type,
    sum_amount,
    subscription_tier,
  } in all_donations
  {
    let points_per_amount = match event_type {
      EventType::Bits => {
        subathon_response.bits += sum_amount as i32;
        POINTS_PER_BIT
      }
      EventType::GiftSubs => {
        let Some(subscription_tier) = subscription_tier else {
          tracing::error!("Missing subscription_tier for gift_sub donation event.");
          continue;
        };

        match subscription_tier {
          1 => {
            subathon_response.tier_1_subs += sum_amount as i32;
            POINTS_PER_TIER_1_SUB
          }
          4 => {
            subathon_response.prime_subs += sum_amount as i32;
            POINTS_PER_TIER_1_SUB
          }
          2 => {
            subathon_response.tier_2_subs += sum_amount as i32;
            POINTS_PER_TIER_2_SUB
          }
          3 => {
            subathon_response.tier_3_subs += sum_amount as i32;
            POINTS_PER_TIER_3_SUB
          }
          _ => {
            tracing::error!("Invalid subscription_tier in gift_sub donation event.");
            continue;
          }
        }
      }
      EventType::StreamlabsDonation => {
        subathon_response.direct_donations += sum_amount;
        POINTS_PER_DOLLAR
      }
    };

    let points = points_per_amount * sum_amount;

    subathon_response.total_points += points;
  }

  let rounded_donations = round_to_100ths(subathon_response.direct_donations);
  let rounded_points = subathon_response.total_points.round();

  subathon_response.direct_donations = rounded_donations;
  subathon_response.total_points = rounded_points;

  Ok(axum::Json(subathon_response))
}

#[derive(Debug, FromQueryResult)]
pub struct DonationSum {
  pub event_type: EventType,
  pub sum_amount: f64,
  pub subscription_tier: Option<i32>,
}

#[derive(Debug, FromQueryResult)]
pub struct StrippedSubscriptionEvent {
  pub subscription_tier: i32,
}

impl From<StrippedSubscriptionEvent> for DonationSum {
  fn from(value: StrippedSubscriptionEvent) -> Self {
    DonationSum {
      event_type: EventType::GiftSubs,
      sum_amount: 1.0,
      subscription_tier: Some(value.subscription_tier),
    }
  }
}

fn round_to_100ths(number: f64) -> f64 {
  (number * 100.0).round() / 100.0
}
