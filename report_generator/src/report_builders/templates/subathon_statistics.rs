use crate::{
  conditions::query_conditions::AppQueryConditions, errors::AppError,
  report_builders::templates::subathon_statistics::donation_sum::StrippedSubscriptionEvent,
};
use chrono::{Duration as ChronoDuration, Utc};
use donation_sum::DonationSum;
use entities::{donation_event, sea_orm_active_enums::EventType, stream, subscription_event};
use sea_orm::*;

mod donation_sum;

const POINTS_PER_BIT: f64 = 0.01;
const POINTS_PER_TIER_1_SUB: f64 = 5.0;
const POINTS_PER_TIER_2_SUB: f64 = 8.0;
const POINTS_PER_TIER_3_SUB: f64 = 20.0;
const POINTS_PER_DOLLAR: f64 = 1.0;

const SECONDS_PER_POINT: f64 = 6.0;

#[derive(Debug, serde::Serialize)]
pub struct SubathonStatistics {
  hours_streamed: f64,
  hours_added_from_donations: f64,
  total_points: i32,
}

impl SubathonStatistics {
  pub const NAME: &str = "subathon_stats";

  pub async fn new(
    query_conditions: &AppQueryConditions,
    database_connection: &DatabaseConnection,
  ) -> Result<Self, AppError> {
    tracing::info!("Building subathon statistics.");

    let total_stream_duration =
      Self::get_duration_of_streams(query_conditions, database_connection).await?;
    let total_stream_duration = total_stream_duration.as_seconds_f64() / 3600.0;
    let points_from_donations =
      Self::points_from_donations(query_conditions, database_connection).await?;
    let hours_added_from_donations = (SECONDS_PER_POINT * points_from_donations) / 3600.0;

    Ok(Self {
      hours_streamed: total_stream_duration,
      hours_added_from_donations,
      total_points: points_from_donations as i32,
    })
  }

  async fn get_duration_of_streams(
    query_conditions: &AppQueryConditions,
    database_connection: &DatabaseConnection,
  ) -> Result<ChronoDuration, AppError> {
    tracing::info!("Calculating duration of all streams.");

    let streams = stream::Entity::find()
      .filter(query_conditions.streams().clone())
      .all(database_connection)
      .await?;

    Ok(
      streams
        .into_iter()
        .filter_map(|stream| {
          let Some(start_time) = stream.start_timestamp else {
            tracing::error!( "Failed to get stream start_time for stream ID `{}`", stream.id);
            return None;
          };
          let end_time = stream.end_timestamp.unwrap_or(Utc::now());

          if start_time > end_time {
            tracing::error!("Found a stream where the end time `{end_time:?}` is older than the start time `{start_time:?}`");
          }

          Some(end_time - start_time)
        })
        .sum(),
    )
  }

  pub async fn points_from_donations(
    query_conditions: &AppQueryConditions,
    database_connection: &DatabaseConnection,
  ) -> Result<f64, AppError> {
    tracing::info!("Calculating total points from all donations since start time.");

    let mut all_donations = donation_event::Entity::find()
      .filter(query_conditions.donations().clone())
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
      .filter(query_conditions.subscriptions().clone())
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

    let mut total_points = 0.0_f64;

    for DonationSum {
      event_type,
      sum_amount,
      subscription_tier,
    } in all_donations
    {
      let points_per_amount = match event_type {
        EventType::Bits => POINTS_PER_BIT,
        EventType::GiftSubs => {
          let Some(subscription_tier) = subscription_tier else {
            tracing::error!("Missing subscription_tier for gift_sub donation event.");
            continue;
          };

          match subscription_tier {
            1 => POINTS_PER_TIER_1_SUB,
            2 => POINTS_PER_TIER_2_SUB,
            3 => POINTS_PER_TIER_3_SUB,
            _ => {
              tracing::error!("Invalid subscription_tier in gift_sub donation event.");
              continue;
            }
          }
        }
        EventType::StreamlabsDonation => POINTS_PER_DOLLAR,
      };

      let points = points_per_amount * sum_amount;

      total_points += points;
    }

    Ok(total_points)
  }
}
