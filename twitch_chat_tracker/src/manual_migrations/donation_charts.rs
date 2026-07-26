use chrono::{DateTime, Datelike, Utc};
use database_connection::get_database_connection;
use entities::sea_orm_active_enums::EventType;
use entities::{donation_event, subscription_event, twitch_user};
use entity_extensions::twitch_user::ChannelIdentifier;
use entity_extensions::twitch_user::*;
use sea_orm::*;
use std::collections::HashMap;
use tokio::fs;
use twitch_chat_tracker::errors::AppError;

const USD_PER_GBP: f32 = 0.76;

const TIER_1: i32 = 1;
const TIER_2: i32 = 2;
const TIER_3: i32 = 3;
const PRIME_TIER: i32 = 4;
const TIER_1_VALUE: f32 = 6.0 * 0.7;
const TIER_2_VALUE: f32 = 10.0 * 0.7;
const TIER_3_VALUE: f32 = 25.0 * 0.7;
const PRIME_TIER_VALUE: f32 = TIER_1_VALUE;

const DONATION_ROW_HEADERS: &str = "year-month\tvalue";
const ALL_DONATIONS_NAME: &str = "AllDonations";

const OUTPUT_DIRECTORY: &str = "donation_tables";

#[derive(Debug)]
struct DonationRow {
  timestamp: DonationTimestamp,
  value: f32,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Ord, PartialOrd)]
struct DonationTimestamp {
  year: i32,
  month: u32,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct DonationRowKey {
  timestamp: DonationTimestamp,
  event_type: DonationEventType,
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Hash)]
enum DonationEventType {
  Bits,
  GiftSubs,
  Streamlabs,
  Subscription,
}

struct RowConfigurations<'a> {
  by_type_and_timestamp: &'a HashMap<DonationEventType, Vec<DonationRow>>,
  by_timestamp: Vec<DonationRow>,
}

struct DonationTable {
  name: String,
  data: String,
}

impl From<EventType> for DonationEventType {
  fn from(event_type: EventType) -> Self {
    match event_type {
      EventType::Bits => DonationEventType::Bits,
      EventType::GiftSubs => DonationEventType::GiftSubs,
      EventType::StreamlabsDonation => DonationEventType::Streamlabs,
    }
  }
}

impl std::fmt::Display for DonationTimestamp {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(formatter, "{}-{}", self.year, self.month)
  }
}

pub async fn run() -> ! {
  if let Err(error) =
    create_donation_charts_for_channel(ChannelIdentifier::Login("fallenshadow")).await
  {
    tracing::error!("Failed to calculate subathon graph. Reason: {error}");

    std::process::exit(1)
  }

  std::process::exit(0);
}

async fn create_donation_charts_for_channel(
  channel_identifier: ChannelIdentifier<&'static str>,
) -> Result<(), AppError> {
  tracing::info!("Building initial state for donation charts for user {channel_identifier:?}...");

  let database_connection = get_database_connection().await;
  let streamer = twitch_user::Model::get_by_identifier(channel_identifier, database_connection)
    .await?
    .unwrap();

  let donation_list = get_donations_for_streamer(&streamer, database_connection).await?;
  let subscription_list = get_subscriptions_for_streamer(&streamer, database_connection).await?;

  let mut donation_rows = build_donation_rows(donation_list, subscription_list);
  sort_donation_rows(&mut donation_rows);

  let row_configurations = RowConfigurations {
    by_type_and_timestamp: &donation_rows,
    by_timestamp: flatten_donation_rows(&donation_rows),
  };

  let donation_tables = build_tables(&row_configurations);

  if !write_tables_to_file(donation_tables).await {
    tracing::error!("! Failed to write all tables to file !");
  }

  Ok(())
}

fn build_tables(row_configurations: &RowConfigurations) -> Vec<DonationTable> {
  let mut donation_tables = vec![];

  row_configurations
    .by_type_and_timestamp
    .iter()
    .for_each(|(event_type, donation_rows)| {
      let table = DonationTable {
        name: format!("{:?}", event_type),
        data: table_from_rows(donation_rows),
      };

      donation_tables.push(table);
    });

  let all_types_table = DonationTable {
    name: String::from(ALL_DONATIONS_NAME),
    data: table_from_rows(row_configurations.by_timestamp.iter()),
  };

  donation_tables.push(all_types_table);

  donation_tables
}

fn table_from_rows<'a>(donation_rows: impl IntoIterator<Item = &'a DonationRow>) -> String {
  donation_rows.into_iter().fold(
    format!("{DONATION_ROW_HEADERS}\n"),
    |mut donation_table, donation_row| {
      let row = format!("{}\t{:.2}\n", donation_row.timestamp, donation_row.value);
      donation_table.push_str(&row);

      donation_table
    },
  )
}

/// Returns true if the process finished successfully.
///
/// # Panics
///
/// - If the OUTPUT_DIRECTORY couldn't be built.
async fn write_tables_to_file(donation_tables: Vec<DonationTable>) -> bool {
  let mut process_succeeded = true;

  fs::create_dir_all(OUTPUT_DIRECTORY).await.unwrap();

  for DonationTable {
    name,
    data: table_data,
  } in donation_tables
  {
    let file_path = format!("{OUTPUT_DIRECTORY}/{name}.txt");

    tracing::info!("Writing data for the `{name}` table at `{file_path}`");

    if let Err(error) = fs::write(&file_path, table_data).await {
      tracing::error!("Failed to write to `{file_path}`. Reason: `{error}`");

      process_succeeded = false;
    }
  }

  process_succeeded
}

async fn get_subscriptions_for_streamer(
  streamer: &twitch_user::Model,
  database_connection: &DatabaseConnection,
) -> Result<Vec<subscription_event::Model>, AppError> {
  tracing::info!("Getting subscriptions for streamer `{streamer:?}`...");

  subscription_event::Entity::find()
    .filter(subscription_event::Column::ChannelId.eq(streamer.id))
    .all(database_connection)
    .await
    .map_err(Into::into)
}

fn build_donation_rows(
  donations: Vec<donation_event::Model>,
  subscriptions: Vec<subscription_event::Model>,
) -> HashMap<DonationEventType, Vec<DonationRow>> {
  tracing::info!("Building donation rows");

  let mut donation_rows: HashMap<DonationRowKey, DonationRow> = HashMap::new();

  donations.into_iter().for_each(|donation_event| {
    let value = get_donation_value(&donation_event);
    let timestamp = convert_timestamp_by_week(donation_event.timestamp);
    let event_type: DonationEventType = donation_event.event_type.into();

    let key = DonationRowKey {
      timestamp,
      event_type,
    };

    let entry = donation_rows.entry(key).or_insert(DonationRow {
      timestamp,
      value: 0.0,
    });

    entry.value += value;
  });

  subscriptions.into_iter().for_each(|subscription_event| {
    let value = get_subscription_value(&subscription_event);
    let timestamp = convert_timestamp_by_week(subscription_event.timestamp);
    let event_type = DonationEventType::Subscription;

    let key = DonationRowKey {
      timestamp,
      event_type,
    };

    let entry = donation_rows.entry(key).or_insert(DonationRow {
      timestamp,
      value: 0.0,
    });

    entry.value += value;
  });

  donation_rows
    .into_iter()
    .fold(HashMap::new(), |mut donation_rows, (key, donation_row)| {
      let entry = donation_rows.entry(key.event_type).or_default();
      entry.push(donation_row);

      donation_rows
    })
}

fn convert_timestamp_by_week(timestamp: DateTime<Utc>) -> DonationTimestamp {
  DonationTimestamp {
    year: timestamp.year(),
    month: timestamp.month(),
  }
}

/// Sorts the final list by timestamp.
fn flatten_donation_rows(
  donation_rows: &HashMap<DonationEventType, Vec<DonationRow>>,
) -> Vec<DonationRow> {
  let donation_list_by_timestamp: HashMap<DonationTimestamp, DonationRow> =
    donation_rows.iter().fold(
      HashMap::new(),
      |mut final_list, (_event_type, donation_rows)| {
        donation_rows.iter().for_each(|donation_row| {
          let timestamp = donation_row.timestamp;
          let entry = final_list.entry(timestamp).or_insert(DonationRow {
            timestamp,
            value: 0.0,
          });

          entry.value += donation_row.value;
        });

        final_list
      },
    );

  let mut flat_donation_list: Vec<DonationRow> = donation_list_by_timestamp.into_values().collect();
  flat_donation_list.sort_by(|lhs, rhs| lhs.timestamp.cmp(&rhs.timestamp));

  println!("{flat_donation_list:#?}");

  flat_donation_list
}

fn sort_donation_rows(donation_rows: &mut HashMap<DonationEventType, Vec<DonationRow>>) {
  donation_rows
    .iter_mut()
    .for_each(|(_event_type, donation_rows)| {
      donation_rows.sort_by(|lhs, rhs| lhs.timestamp.cmp(&rhs.timestamp));
    });
}

async fn get_donations_for_streamer(
  streamer: &twitch_user::Model,
  database_connection: &DatabaseConnection,
) -> Result<Vec<donation_event::Model>, AppError> {
  tracing::info!("Getting donations for streamer `{streamer:?}`...");

  donation_event::Entity::find()
    .filter(donation_event::Column::DonationReceiverTwitchUserId.eq(streamer.id))
    .all(database_connection)
    .await
    .map_err(Into::into)
}

fn get_donation_value(donation_event: &donation_event::Model) -> f32 {
  match donation_event.event_type {
    EventType::Bits => donation_event.amount / 100.0 * USD_PER_GBP,
    EventType::StreamlabsDonation => donation_event.amount,
    EventType::GiftSubs => match donation_event.subscription_tier {
      Some(TIER_1) => TIER_1_VALUE,
      Some(TIER_2) => TIER_2_VALUE,
      Some(TIER_3) => TIER_3_VALUE,
      _ => {
        tracing::error!(
          "Failed to find subscription tier for gift sub donation event {donation_event:?}"
        );

        0.0
      }
    },
  }
}

fn get_subscription_value(subscription_event: &subscription_event::Model) -> f32 {
  match subscription_event.subscription_tier {
    Some(PRIME_TIER) => PRIME_TIER_VALUE,
    Some(TIER_1) => TIER_1_VALUE,
    Some(TIER_2) => TIER_2_VALUE,
    Some(TIER_3) => TIER_3_VALUE,
    _ => {
      tracing::error!("Invalid subscription tier for subscription event {subscription_event:?}");

      0.0
    }
  }
}
