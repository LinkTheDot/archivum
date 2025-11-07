use chrono::{DateTime, Duration as ChronoDuration, NaiveDateTime, TimeZone, Utc};
use database_connection::get_database_connection;
use entities::{donation_event, sea_orm_active_enums::EventType, stream, subscription_event};
use sea_orm::*;
use std::collections::HashSet;
use twitch_chat_tracker::errors::AppError;

const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

const TWITCH_USER_ID: i32 = 1;
const STARTING_STREAM_ID: i32 = 239;
const ENDING_STREAM_ID: i32 = 260;
const TIME_STEP: ChronoDuration = ChronoDuration::seconds(120);

const POINTS_PER_BIT: f32 = 0.01;
const POINTS_PER_TIER_1_SUB: f32 = 5.0;
const POINTS_PER_TIER_2_SUB: f32 = 10.0;
const POINTS_PER_TIER_3_SUB: f32 = 25.0;
const POINTS_PER_DOLLAR: f32 = 1.0;

const TIME_PER_POINT: ChronoDuration = ChronoDuration::seconds(6);

const HYPETRAIN_TIME_ADDED: (i64, &str) = (12, "2025-10-19 17:35:59");
const PAUSED_TIMER: &[(&str, &str)] = &[("2025-10-30 18:50:36", "2025-10-30 22:35:26")];
const START_TIME: i64 = 8;

#[allow(unused)]
struct Row {
  stream_day: i32,
  time_added: ChronoDuration,
  points_this_timeframe: i32,
  current_timer: ChronoDuration,
  current_time: DateTime<Utc>,
}

#[derive(Default)]
struct CountedDonations {
  bits: f32,
  direct_donations: f32,
  tier_1_subs: f32,
  tier_2_subs: f32,
  tier_3_subs: f32,
}

struct StoredDonations {
  donation_events: Vec<donation_event::Model>,
  subscription_events: Vec<subscription_event::Model>,

  used_donation_ids: HashSet<i32>,
  used_subscription_ids: HashSet<i32>,
}

#[allow(unused)]
pub async fn calculate_subathon_graph() -> ! {
  if let Err(error) = run().await {
    tracing::error!("Failed to calculate subathon graph. Reason: {error}");

    std::process::exit(1)
  }

  std::process::exit(0)
}

async fn run() -> Result<(), AppError> {
  let database_connection = get_database_connection().await;
  let streams = get_streams(database_connection).await?;
  let start_time = streams[0].start_timestamp.unwrap();
  let end_time = streams.last().unwrap().end_timestamp.unwrap_or(Utc::now());

  assert!(start_time < end_time);

  let mut current_time = start_time;
  let mut rows: Vec<Row> = vec![];
  let mut stored_donations =
    StoredDonations::new(database_connection, start_time, end_time).await?;

  let add_hypetrain_time_after = time_from_string(HYPETRAIN_TIME_ADDED.1);
  let paused_times: Vec<(DateTime<Utc>, DateTime<Utc>)> = PAUSED_TIMER
    .iter()
    .map(|(start, end)| {
      let start = time_from_string(start);
      let end = time_from_string(end);

      (start, end)
    })
    .collect();

  let added_start_time = ChronoDuration::hours(START_TIME);
  let mut hypetrain_time_added = false;

  let mut current_timer = ChronoDuration::from(added_start_time);

  while current_time <= end_time {
    let Some(stream) = time_is_in_streams(&streams, current_time) else {
      current_time += TIME_STEP;
      continue;
    };

    let timer_is_paused = paused_times
      .iter()
      .any(|(start, end)| &current_time >= start && &current_time <= end);

    if !timer_is_paused {
      current_timer -= TIME_STEP;
    }

    let counted_donations = stored_donations
      .count_donations_up_to_time(current_time)
      .await?;

    let points = counted_donations.into_points();
    let time_added = TIME_PER_POINT * points;

    current_timer += time_added;

    if !hypetrain_time_added && current_time >= add_hypetrain_time_after {
      current_timer += ChronoDuration::hours(HYPETRAIN_TIME_ADDED.0);
      hypetrain_time_added = true;
    }

    let row = Row {
      stream_day: stream,
      time_added,
      points_this_timeframe: points,
      current_timer,
      current_time,
    };

    rows.push(row);

    current_time += TIME_STEP;
  }

  let table = build_table(rows);
  // let table = build_yatou_table(rows);

  tokio::fs::write("output.txt", table).await.unwrap();

  Ok(())
}

#[allow(unused)]
fn build_table(rows: Vec<Row>) -> String {
  let max_row = rows.last().unwrap().stream_day;

  let mut header = "Stream Day\tHours Passed".to_string();

  for day_number in 1..=max_row {
    header.push_str(&format!("\tDay {}", day_number))
  }

  let body = rows
    .into_iter()
    .enumerate()
    .map(|(iteration, row)| {
      let day_spacer = "\t".repeat(row.stream_day as usize);
      let hours_passed = TIME_STEP * iteration as i32;

      format!(
        "{}\t{:.2}{day_spacer}{:.2}",
        row.stream_day,
        hours_passed.num_seconds() as f64 / 3600.0,
        row.current_timer.num_seconds() as f64 / 3600.0,
      )
    })
    .collect::<Vec<String>>()
    .join("\n");

  format!("{header}\n{body}")
}

#[allow(unused)]
fn build_yatou_table(rows: Vec<Row>) -> String {
  let header = "DateTime\tTimer\tTimeAddedByDonations";

  let body = rows
    .into_iter()
    .enumerate()
    .map(|(iteration, row)| {
      let hours_passed = TIME_STEP * iteration as i32;
      let current_time = row.current_time.format("%Y-%m-%d %H:%M:%S").to_string();

      format!(
        "{}\t{:.2}\t{:.2}",
        current_time,
        row.current_timer.num_seconds() as f64 / 3600.0,
        row.points_this_timeframe * TIME_PER_POINT.num_seconds() as i32,
      )
    })
    .collect::<Vec<String>>()
    .join("\n");

  format!("{header}\n{body}")
}

fn time_is_in_streams(streams: &[stream::Model], time: DateTime<Utc>) -> Option<i32> {
  streams
    .iter()
    .position(|stream| {
      let Some(start_time) = stream.start_timestamp else {
        tracing::error!("Failed to find start time for stream `{}`", stream.id);
        return false;
      };
      let end_time = stream.end_timestamp.unwrap_or(Utc::now());
      time <= end_time && time >= start_time
    })
    .map(|index| index as i32 + 1)
}

async fn get_streams(
  database_connection: &DatabaseConnection,
) -> Result<Vec<stream::Model>, AppError> {
  stream::Entity::find()
    .filter(stream::Column::TwitchUserId.eq(TWITCH_USER_ID))
    .filter(stream::Column::Id.between(STARTING_STREAM_ID, ENDING_STREAM_ID))
    .all(database_connection)
    .await
    .map_err(Into::into)
}

impl StoredDonations {
  async fn new(
    database_connection: &DatabaseConnection,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
  ) -> Result<Self, AppError> {
    let donations = donation_event::Entity::find()
      .filter(donation_event::Column::DonationReceiverTwitchUserId.eq(TWITCH_USER_ID))
      .filter(donation_event::Column::Timestamp.between(start_time, end_time))
      .all(database_connection)
      .await?;
    let subscriptions = subscription_event::Entity::find()
      .filter(subscription_event::Column::ChannelId.eq(TWITCH_USER_ID))
      .filter(subscription_event::Column::Timestamp.between(start_time, end_time))
      .all(database_connection)
      .await?;

    Ok(Self {
      donation_events: donations,
      subscription_events: subscriptions,
      used_donation_ids: HashSet::new(),
      used_subscription_ids: HashSet::new(),
    })
  }

  async fn count_donations_up_to_time(
    &mut self,
    end_time: DateTime<Utc>,
  ) -> Result<CountedDonations, AppError> {
    let donations = self.donations_up_to_time(end_time).await;
    let subscriptions = self.subscriptions_up_to_time(end_time).await;
    let mut counted_donations = CountedDonations::default();
    let donation_ids: Vec<i32> = donations.iter().map(|d| d.id).collect();
    let subscription_ids: Vec<i32> = subscriptions.iter().map(|s| s.id).collect();

    donations.iter().for_each(|donation_event| {
      counted_donations.add_from_donation_event(donation_event);
    });
    subscriptions.iter().for_each(|subscription_event| {
      counted_donations.add_from_subscription_event(subscription_event);
    });

    self.used_donation_ids.extend(donation_ids.iter());
    self.used_subscription_ids.extend(subscription_ids.iter());

    Ok(counted_donations)
  }

  async fn donations_up_to_time(&self, end: DateTime<Utc>) -> Vec<&donation_event::Model> {
    self
      .donation_events
      .iter()
      .filter(|donation| {
        donation.timestamp <= end && !self.used_donation_ids.contains(&donation.id)
      })
      .collect()
  }

  async fn subscriptions_up_to_time(&self, end: DateTime<Utc>) -> Vec<&subscription_event::Model> {
    self
      .subscription_events
      .iter()
      .filter(|subscription| {
        subscription.timestamp <= end && !self.used_subscription_ids.contains(&subscription.id)
      })
      .collect()
  }
}

fn time_from_string(time_string: &str) -> DateTime<Utc> {
  let naive_date_time = NaiveDateTime::parse_from_str(time_string, TIME_FORMAT).unwrap();

  Utc.from_utc_datetime(&naive_date_time)
}

impl CountedDonations {
  fn add_from_donation_event(&mut self, donation_event: &donation_event::Model) {
    let subscription_tier = donation_event.subscription_tier.unwrap_or(10000);
    let amount = donation_event.amount;

    match donation_event.event_type {
      EventType::Bits => self.bits += amount,
      EventType::StreamlabsDonation => self.direct_donations += amount,
      EventType::GiftSubs if subscription_tier == 1 => self.tier_1_subs += amount,
      EventType::GiftSubs if subscription_tier == 2 => self.tier_2_subs += amount,
      EventType::GiftSubs if subscription_tier == 3 => self.tier_3_subs += amount,
      EventType::GiftSubs if subscription_tier == 4 => self.tier_1_subs += amount,
      EventType::GiftSubs => {
        tracing::error!(
          "Invalid subscription_tier for donation_event `{}`.",
          donation_event.id
        )
      }
    };
  }

  fn add_from_subscription_event(&mut self, subscription_event: &subscription_event::Model) {
    let subscription_tier = subscription_event.subscription_tier.unwrap_or(10000);

    match subscription_tier {
      1 | 4 => self.tier_1_subs += 1.0,
      2 => self.tier_2_subs += 1.0,
      3 => self.tier_3_subs += 1.0,
      _ => tracing::error!(
        "Invalid subscription_tier for subscription_event `{}`.",
        subscription_event.id
      ),
    }
  }

  fn into_points(self) -> i32 {
    (self.bits * POINTS_PER_BIT
      + self.direct_donations * POINTS_PER_DOLLAR
      + self.tier_1_subs * POINTS_PER_TIER_1_SUB
      + self.tier_2_subs * POINTS_PER_TIER_2_SUB
      + self.tier_3_subs * POINTS_PER_TIER_3_SUB)
      .round() as i32
  }
}
