use crate::errors::AppError;
use chrono::{DateTime, Datelike, Months, NaiveDate, NaiveTime, Utc};
use entities::*;
use sea_orm::Condition;
use sea_orm::*;

#[derive(Debug)]
pub struct AppQueryConditions {
  pub messages: Condition,
  pub timeouts: Condition,
  pub donations: Condition,
  pub subscriptions: Condition,
  pub raids: Condition,
  pub streams: Condition,

  pub stream_id: Option<i32>,
  pub date_start: Option<DateTime<Utc>>,
  pub date_end: Option<DateTime<Utc>>,
  pub streamer_twitch_user_id: Option<i32>,
}

impl AppQueryConditions {
  pub fn from_stream_id(stream_id: i32) -> Self {
    Self {
      messages: Condition::all().add(stream_message::Column::StreamId.eq(Some(stream_id))),
      timeouts: Condition::all().add(user_timeout::Column::StreamId.eq(Some(stream_id))),
      donations: Condition::all().add(donation_event::Column::StreamId.eq(Some(stream_id))),
      subscriptions: Condition::all().add(subscription_event::Column::StreamId.eq(Some(stream_id))),
      raids: Condition::all().add(raid::Column::StreamId.eq(Some(stream_id))),
      streams: Condition::all().add(stream::Column::Id.eq(stream_id)),

      stream_id: Some(stream_id),
      date_start: None,
      date_end: None,
      streamer_twitch_user_id: None,
    }
  }

  pub fn from_month(month: Option<usize>, streamer_twitch_user_id: i32) -> Result<Self, AppError> {
    let (start_date, end_date) = get_month_range(month)?;

    Ok(Self {
      messages: Condition::all()
        .add(stream_message::Column::Timestamp.between(start_date, end_date))
        .add(stream_message::Column::ChannelId.eq(streamer_twitch_user_id)),

      timeouts: Condition::all()
        .add(user_timeout::Column::Timestamp.between(start_date, end_date))
        .add(user_timeout::Column::ChannelId.eq(streamer_twitch_user_id)),

      donations: Condition::all()
        .add(donation_event::Column::Timestamp.between(start_date, end_date))
        .add(donation_event::Column::DonationReceiverTwitchUserId.eq(streamer_twitch_user_id)),

      subscriptions: Condition::all()
        .add(subscription_event::Column::Timestamp.between(start_date, end_date))
        .add(subscription_event::Column::ChannelId.eq(streamer_twitch_user_id)),

      raids: Condition::all()
        .add(raid::Column::Timestamp.between(start_date, end_date))
        .add(raid::Column::TwitchUserId.eq(streamer_twitch_user_id)),

      streams: Condition::all()
        .add(stream::Column::StartTimestamp.gte(start_date))
        .add(stream::Column::EndTimestamp.lte(end_date)),

      stream_id: None,
      date_start: Some(start_date),
      date_end: Some(end_date),
      streamer_twitch_user_id: None,
    })
  }

  pub fn messages(&self) -> &Condition {
    &self.messages
  }

  pub fn timeouts(&self) -> &Condition {
    &self.timeouts
  }

  pub fn donations(&self) -> &Condition {
    &self.donations
  }

  pub fn subscriptions(&self) -> &Condition {
    &self.subscriptions
  }

  pub fn raids(&self) -> &Condition {
    &self.raids
  }

  pub fn streams(&self) -> &Condition {
    &self.streams
  }
}

/// Returns the start and end times for the given month. The current month is used if `None` is passed in.
pub fn get_month_range(month: Option<usize>) -> Result<(DateTime<Utc>, DateTime<Utc>), AppError> {
  let current_time = Utc::now();

  let month = if let Some(month) = month {
    if month == 0 || month > 12 {
      return Err(AppError::InvalidMonthValue(month as i32));
    }

    month
  } else {
    current_time.month() as usize
  };

  let Some(start_date) = NaiveDate::from_ymd_opt(current_time.year(), month as u32, 1) else {
    return Err(AppError::InvalidMonthValue(month as i32));
  };
  let Some(first_of_next_month) = start_date.checked_add_months(Months::new(1)) else {
    return Err(AppError::InvalidMonthValue(month as i32));
  };

  let start_date =
    DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_time(NaiveTime::MIN), Utc);
  let first_of_next_month =
    DateTime::<Utc>::from_naive_utc_and_offset(first_of_next_month.and_time(NaiveTime::MIN), Utc);

  Ok((start_date, first_of_next_month))
}

#[cfg(test)]
mod tests {
  use crate::conditions::query_conditions_builder::AppQueryConditionsBuilder;

  use super::*;
  use chrono::TimeZone;

  #[test]
  fn get_month_day_boundaries_expected_value() {
    let month = 2;
    let now = Utc::now();
    let expected_start = Utc.with_ymd_and_hms(now.year(), 2, 1, 0, 0, 0).unwrap();
    let expected_end = Utc.with_ymd_and_hms(now.year(), 3, 1, 0, 0, 0).unwrap();

    let (start, end) = get_month_range(Some(month)).unwrap();

    assert_eq!(start, expected_start);
    assert_eq!(end, expected_end);
  }

  fn get_expected_datetime_range(
    start_month: usize,
    end_month: usize,
  ) -> (DateTime<Utc>, DateTime<Utc>) {
    let (start_date, _) = get_month_range(Some(start_month)).unwrap();
    let (end_date, _) = get_month_range(Some(end_month)).unwrap();
    (start_date, end_date)
  }

  #[test]
  fn test_stream_message_full_conditions() {
    let (expected_start, expected_end) = get_expected_datetime_range(1, 12);
    let builder = AppQueryConditionsBuilder::new()
      .set_stream_id(123)
      .set_month_range(1, 12)
      .unwrap()
      .set_streamer_twitch_user_id(456);

    let expected_condition = Condition::all()
      .add(stream_message::Column::StreamId.eq(123))
      .add(stream_message::Column::Timestamp.between(expected_start, expected_end))
      .add(stream_message::Column::ChannelId.eq(456));

    let condition = builder.stream_message();

    assert_eq!(condition, expected_condition);
  }
  #[test]
  fn test_stream_message_only_stream_id() {
    let builder = AppQueryConditionsBuilder::new().set_stream_id(123);

    let expected_condition = Condition::all().add(stream_message::Column::StreamId.eq(Some(123)));

    let condition = builder.stream_message();

    assert_eq!(condition, expected_condition);
  }

  #[test]
  fn test_stream_message_only_timestamp() {
    let (expected_start, expected_end) = get_expected_datetime_range(3, 4);
    let builder = AppQueryConditionsBuilder::new()
      .set_month_range(3, 4)
      .unwrap();

    let expected_condition =
      Condition::all().add(stream_message::Column::Timestamp.between(expected_start, expected_end));

    let condition = builder.stream_message();

    assert_eq!(condition, expected_condition);
  }

  #[test]
  fn test_stream_message_only_user_id() {
    let builder = AppQueryConditionsBuilder::new().set_streamer_twitch_user_id(789);

    let expected_condition = Condition::all().add(stream_message::Column::ChannelId.eq(789));

    let condition = builder.stream_message();

    assert_eq!(condition, expected_condition);
  }

  #[test]
  fn test_stream_message_no_conditions() {
    let builder = AppQueryConditionsBuilder::new();

    let expected_condition = Condition::all();

    let condition = builder.stream_message();

    assert_eq!(condition, expected_condition);
  }

  #[test]
  fn test_user_timeout_full_conditions() {
    let (expected_start, expected_end) = get_expected_datetime_range(5, 6);
    let builder = AppQueryConditionsBuilder::new()
      .set_stream_id(100)
      .set_month_range(5, 6)
      .unwrap()
      .set_streamer_twitch_user_id(200);

    let expected_condition = Condition::all()
      .add(user_timeout::Column::StreamId.eq(Some(100)))
      .add(user_timeout::Column::Timestamp.between(expected_start, expected_end))
      .add(user_timeout::Column::ChannelId.eq(200));

    let condition = builder.user_timeout();

    assert_eq!(condition, expected_condition);
  }

  #[test]
  fn test_user_timeout_only_stream_id() {
    let builder = AppQueryConditionsBuilder::new().set_stream_id(100);

    let expected_condition = Condition::all().add(user_timeout::Column::StreamId.eq(Some(100)));

    let condition = builder.user_timeout();

    assert_eq!(condition, expected_condition);
  }

  #[test]
  fn test_user_timeout_no_conditions() {
    let builder = AppQueryConditionsBuilder::new();

    let expected_condition = Condition::all();

    let condition = builder.user_timeout();

    assert_eq!(condition, expected_condition);
  }

  #[test]
  fn test_donation_event_full_conditions() {
    let (expected_start, expected_end) = get_expected_datetime_range(7, 8);
    let builder = AppQueryConditionsBuilder::new()
      .set_stream_id(300)
      .set_month_range(7, 8)
      .unwrap()
      .set_streamer_twitch_user_id(400);

    let expected_condition = Condition::all()
      .add(donation_event::Column::StreamId.eq(Some(300)))
      .add(donation_event::Column::Timestamp.between(expected_start, expected_end))
      .add(donation_event::Column::DonationReceiverTwitchUserId.eq(400));

    let condition = builder.donation_event();

    assert_eq!(condition, expected_condition);
  }

  #[test]
  fn test_donation_event_only_user_id() {
    let builder = AppQueryConditionsBuilder::new().set_streamer_twitch_user_id(400);

    let expected_condition =
      Condition::all().add(donation_event::Column::DonationReceiverTwitchUserId.eq(400));

    let condition = builder.donation_event();

    assert_eq!(condition, expected_condition);
  }

  #[test]
  fn test_donation_event_no_conditions() {
    let builder = AppQueryConditionsBuilder::new();
    let expected_condition = Condition::all();
    let condition = builder.donation_event();
    assert_eq!(condition, expected_condition);
  }

  #[test]
  fn test_subscription_event_full_conditions() {
    let (expected_start, expected_end) = get_expected_datetime_range(9, 10);
    let builder = AppQueryConditionsBuilder::new()
      .set_stream_id(500)
      .set_month_range(9, 10)
      .unwrap()
      .set_streamer_twitch_user_id(600);

    let expected_condition = Condition::all()
      .add(subscription_event::Column::StreamId.eq(Some(500)))
      .add(subscription_event::Column::Timestamp.between(expected_start, expected_end))
      .add(subscription_event::Column::ChannelId.eq(600));

    let condition = builder.subscription_event();

    assert_eq!(condition, expected_condition);
  }

  #[test]
  fn test_subscription_event_no_conditions() {
    let builder = AppQueryConditionsBuilder::new();
    let expected_condition = Condition::all();
    let condition = builder.subscription_event();
    assert_eq!(condition, expected_condition);
  }

  #[test]
  fn test_raid_full_conditions() {
    let (expected_start, expected_end) = get_expected_datetime_range(11, 12);
    let builder = AppQueryConditionsBuilder::new()
      .set_stream_id(700)
      .set_month_range(11, 12)
      .unwrap()
      .set_streamer_twitch_user_id(800);

    let expected_condition = Condition::all()
      .add(raid::Column::StreamId.eq(Some(700)))
      .add(raid::Column::Timestamp.between(expected_start, expected_end))
      .add(raid::Column::TwitchUserId.eq(800));

    let condition = builder.raid();

    assert_eq!(condition, expected_condition);
  }

  #[test]
  fn test_raid_no_conditions() {
    let builder = AppQueryConditionsBuilder::new();
    let expected_condition = Condition::all();
    let condition = builder.raid();
    assert_eq!(condition, expected_condition);
  }
}
