use crate::clap::Args;
use crate::conditions::query_conditions::AppQueryConditions;
use crate::errors::AppError;
use crate::report_builders::tables::chat_messages::get_messages_sent_ranking;
use crate::report_builders::tables::donation_rankings::get_donation_rankings_for_streamer_and_date;
use crate::report_builders::tables::raids::get_raids_table;
use crate::report_builders::tables::timeouts::get_timeouts_table;
use crate::report_builders::tables::top_emotes::get_top_n_emotes_table;
use crate::report_builders::templates::chat_statistics::ChatStatistics;
use crate::report_builders::templates::template_renderer::TemplateRenderer;
use crate::reports::{Report, Reports};
use chrono::{Datelike, NaiveDate, TimeZone, Utc};
use database_connection::get_database_connection;

const MONTHLY_RANKING_ROW_LIMIT: usize = 1000;
const REPORT_SECTION_SEPARATION: &str = "\n\n";

/// Generates reports for the given streamer with the passed in conditions.
pub async fn generate_reports(
  query_conditions: AppQueryConditions,
  streamer_twitch_user_id: i32,
) -> Result<Reports, AppError> {
  let mut reports = Reports::default();

  let monthly_conditions =
    AppQueryConditions::from_month(Args::get_month(), streamer_twitch_user_id)?;

  let baseline_reports = get_baseline_reports(query_conditions, &monthly_conditions).await?;
  let conditional_reports =
    get_conditional_reports(&monthly_conditions, streamer_twitch_user_id).await?;

  reports.add_reports(baseline_reports);
  reports.add_reports(conditional_reports);

  Ok(reports)
}

/// Gets the reports that will always be added regardless of arguments passed in.
async fn get_baseline_reports(
  query_conditions: AppQueryConditions,
  monthly_conditions: &AppQueryConditions,
) -> Result<Vec<Report>, AppError> {
  let database_connection = get_database_connection().await;
  let mut template_renderer = TemplateRenderer::new();
  let general_chat_statistics = ChatStatistics::new(&query_conditions).await?;
  let monthly_general_chat_statistics = ChatStatistics::new(monthly_conditions).await?;

  template_renderer.add_context(ChatStatistics::NAME, &general_chat_statistics);
  template_renderer
    .add_template_from_template_file_name("general_stats", "general_chat_stats")
    .await?;
  template_renderer
    .add_template_from_template_file_name("donation_stats", "donation_stats")
    .await?;

  let rendered_chat_statistics = template_renderer.render("general_stats")?;
  let rendered_donation_statistics = template_renderer.render("donation_stats")?;
  let top_emotes_table =
    get_top_n_emotes_table(&query_conditions, database_connection, Some(15)).await?;
  let raids = get_raids_table(&query_conditions, database_connection).await?;
  let timeouts = get_timeouts_table(&query_conditions, database_connection).await?;

  template_renderer.add_context(ChatStatistics::NAME, &monthly_general_chat_statistics);

  let monthly_rendered_chat_statistics = template_renderer.render("general_stats")?;
  let monthly_rendered_donation_statistics = template_renderer.render("donation_stats")?;
  let monthly_top_emotes_table =
    get_top_n_emotes_table(monthly_conditions, database_connection, Some(15)).await?;

  let general_stats_report = Report::build_report_from_list(
    "general_stats",
    &[
      &raids,
      &timeouts,
      &top_emotes_table,
      &rendered_chat_statistics,
    ],
    REPORT_SECTION_SEPARATION,
  );
  let monthly_general_stats_report = Report::build_report_from_list(
    "monthly_general_stats",
    &[&monthly_top_emotes_table, &monthly_rendered_chat_statistics],
    REPORT_SECTION_SEPARATION,
  );
  let general_stats_report_with_donations = Report::build_report_from_list(
    "general_stats_with_donations",
    &[
      &raids,
      &timeouts,
      &top_emotes_table,
      &rendered_chat_statistics,
      &rendered_donation_statistics,
    ],
    REPORT_SECTION_SEPARATION,
  );
  let monthly_general_stats_report_with_donations = Report::build_report_from_list(
    "monthly_general_stats_with_donations",
    &[
      &raids,
      &monthly_top_emotes_table,
      &monthly_rendered_chat_statistics,
      &monthly_rendered_donation_statistics,
    ],
    REPORT_SECTION_SEPARATION,
  );
  tracing::info!("Generating chat message rankings.");
  let (unfiltered_chat_report, quality_filtered_chat_report) =
    get_messages_sent_ranking(&query_conditions, None).await?;

  let reports = vec![
    general_stats_report,
    monthly_general_stats_report,
    Report::new("unfiltered_chat_rankings", unfiltered_chat_report),
    Report::new("filtered_chat_rankings", quality_filtered_chat_report),
    general_stats_report_with_donations,
    monthly_general_stats_report_with_donations,
  ];

  Ok(reports)
}

/// Gets the list of reports that will only be added based on passed in flags or available data.
async fn get_conditional_reports(
  monthly_conditions: &AppQueryConditions,
  streamer_twitch_user_id: i32,
) -> Result<Vec<Report>, AppError> {
  let mut conditional_reports = vec![];

  tracing::info!("Generating donation rankings.");
  let current_date = chrono::Local::now();
  let year = Args::get_year().unwrap_or(current_date.year() as usize) as i32;
  let month = Args::get_month().unwrap_or(current_date.month() as usize) as u32;

  let date_start = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
  let date_start = Utc.from_utc_datetime(&date_start.and_hms_opt(0, 0, 0).unwrap());
  let (end_year, end_month) = if month == 12 {
    (year + 1, 1)
  } else {
    (year, month + 1)
  };
  let date_end = NaiveDate::from_ymd_opt(end_year, end_month, 1).unwrap();
  let date_end = Utc.from_utc_datetime(&date_end.and_hms_opt(0, 0, 0).unwrap());

  let donator_monthly_rankings_result =
    get_donation_rankings_for_streamer_and_date(streamer_twitch_user_id, date_start, date_end)
      .await;

  match donator_monthly_rankings_result {
    Ok(donator_monthly_rankings) => conditional_reports.push(Report::new(
      "donator_monthly_rankings",
      donator_monthly_rankings,
    )),
    Err(error) => tracing::error!(
      "Failed to generate monthly donation rankings. Reason: {:?}",
      error
    ),
  }

  if Args::run_monthly_chat_ranking() {
    tracing::info!("Generating monthly chat message rankings.");
    let (monthly_unfiltered_chat_report, monthly_quality_filtered_chat_report) =
      get_messages_sent_ranking(monthly_conditions, Some(MONTHLY_RANKING_ROW_LIMIT)).await?;

    conditional_reports.push(Report::new(
      "monthly_unfiltered_chat_rankings",
      monthly_unfiltered_chat_report,
    ));
    conditional_reports.push(Report::new(
      "monthly_quality_filtered_chat_rankings",
      monthly_quality_filtered_chat_report,
    ));
  }

  Ok(conditional_reports)
}
