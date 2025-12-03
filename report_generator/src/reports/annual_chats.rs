use crate::{
  clap::Args,
  conditions::query_conditions_builder::AppQueryConditionsBuilder,
  errors::AppError,
  report_builders::tables::chat_messages::get_messages_sent_ranking,
  reports::{Report, Reports},
};
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};

const ANNUAL_RANKING_ROW_LIMIT: usize = 1000;

/// Generates reports for the given streamer with the passed in conditions.
pub async fn generate_reports(streamer_twitch_user_id: i32) -> Result<Reports, AppError> {
  let mut reports = Reports::default();
  let (year_start, year_end) = get_year_range().unwrap();

  let annual_conditions = AppQueryConditionsBuilder::new()
    .set_streamer_twitch_user_id(streamer_twitch_user_id)
    .set_time_range(year_start, year_end)?
    .build()?;

  tracing::info!("Generating chat message rankings for annual messages.");
  let (unfiltered_annual_chat_report, annual_emote_filtered_chat_report) =
    get_messages_sent_ranking(&annual_conditions, Some(ANNUAL_RANKING_ROW_LIMIT)).await?;

  let message_reports = vec![
    Report::new(
      "unfiltered_annual_chat_report",
      unfiltered_annual_chat_report,
    ),
    Report::new(
      "annual_emote_filtered_chat_report",
      annual_emote_filtered_chat_report,
    ),
  ];

  reports.add_reports(message_reports);

  Ok(reports)
}

fn get_year_range() -> Option<(DateTime<Utc>, DateTime<Utc>)> {
  let year = Args::get_year().unwrap_or(Utc::now().year() as usize) as i32;

  let year_start = NaiveDate::from_ymd_opt(year, 1, 1)?;
  let year_start = Utc.from_utc_datetime(&year_start.and_hms_opt(0, 0, 0)?);

  let year_end = NaiveDate::from_ymd_opt(year + 1, 1, 1)?;
  let year_end = Utc.from_utc_datetime(&year_end.and_hms_opt(0, 0, 0)?);

  Some((year_start, year_end))
}
