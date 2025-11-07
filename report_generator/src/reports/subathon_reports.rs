use crate::clap::Args;
use crate::conditions::query_conditions::AppQueryConditions;
use crate::conditions::query_conditions_builder::AppQueryConditionsBuilder;
use crate::errors::AppError;
use crate::report_builders::tables::{
  chat_messages::get_messages_sent_ranking,
  donation_rankings::get_donation_rankings_for_streamer_and_date, timeouts::get_timeouts_table,
  top_emotes::get_top_n_emotes_table,
};
use crate::report_builders::templates::subathon_statistics::SubathonStatistics;
use crate::report_builders::templates::{
  chat_statistics::ChatStatistics, template_renderer::TemplateRenderer,
};
use crate::reports::{Report, Reports};
use chrono::Utc;
use database_connection::get_database_connection;

const MONTHLY_RANKING_ROW_LIMIT: usize = 1000;
const SUBATHON_RANKING_ROW_LIMIT: usize = 1000;
const REPORT_SECTION_SEPARATION: &str = "\n\n";

/// Generates reports for the given streamer with the passed in conditions.
pub async fn generate_reports(
  query_conditions: AppQueryConditions,
  streamer_twitch_user_id: i32,
) -> Result<Reports, AppError> {
  let mut reports = Reports::default();
  let Some(subathon_start_date) = Args::subathon_start_date().cloned() else {
    return Err(AppError::MissingSubathonStartTime);
  };
  let subathon_end_date = Args::subathon_end_date().cloned().unwrap_or(Utc::now());

  let monthly_conditions =
    AppQueryConditions::from_month(Args::get_month(), streamer_twitch_user_id)?;
  let subathon_conditions =
    AppQueryConditionsBuilder::copy_from_existing_query_conditions(&query_conditions)
      .set_time_range(subathon_start_date, subathon_end_date)?
      .wipe_stream_id()
      .build()?;

  tracing::info!(
    "Building reports for subathon starting at {:?}.",
    subathon_start_date
  );

  let baseline_reports = get_baseline_reports(&query_conditions, &subathon_conditions).await?;
  let conditional_reports = get_conditional_reports(
    &monthly_conditions,
    &subathon_conditions,
    streamer_twitch_user_id,
  )
  .await?;

  tracing::info!("Finished building reports for subathon.");

  reports.add_reports(baseline_reports);
  reports.add_reports(conditional_reports);

  Ok(reports)
}

/// Gets the reports that will always be added regardless of arguments passed in.
async fn get_baseline_reports(
  query_conditions: &AppQueryConditions,
  subathon_conditions: &AppQueryConditions,
) -> Result<Vec<Report>, AppError> {
  tracing::info!("Generating baseline reports.");

  let database_connection = get_database_connection().await;
  let mut template_renderer = TemplateRenderer::new();
  let subathon_statistics =
    SubathonStatistics::new(subathon_conditions, database_connection).await?;
  let general_chat_statistics = ChatStatistics::new(subathon_conditions).await?;

  tracing::info!("Building template renderer.");
  template_renderer.add_context(ChatStatistics::NAME, &general_chat_statistics);
  template_renderer.add_context(SubathonStatistics::NAME, &subathon_statistics);
  template_renderer
    .add_many_templates_from_files(&[
      (
        "general_stats",
        "report_generator/template_files/general_chat_stats",
      ),
      (
        "donation_stats",
        "report_generator/template_files/donation_stats",
      ),
      (
        "subathon_stats",
        "report_generator/template_files/subathon_stats",
      ),
    ])
    .await?;

  let rendered_chat_statistics = template_renderer.render("general_stats")?;
  let rendered_donation_statistics = template_renderer.render("donation_stats")?;
  let rendered_subathon_statistics = template_renderer.render("subathon_stats")?;
  let top_emotes_table =
    get_top_n_emotes_table(subathon_conditions, database_connection, Some(15)).await?;
  let timeouts = get_timeouts_table(subathon_conditions, database_connection).await?;

  tracing::info!("Building report strings.");

  let general_stats_report = Report::build_report_from_list(
    "subathon_general_stats",
    &[
      &timeouts,
      &top_emotes_table,
      &rendered_chat_statistics,
      &rendered_subathon_statistics,
    ],
    REPORT_SECTION_SEPARATION,
  );
  let general_stats_report_with_donations = Report::build_report_from_list(
    "subathon_general_stats_with_donations",
    &[
      &timeouts,
      &top_emotes_table,
      &rendered_chat_statistics,
      &rendered_subathon_statistics,
      &rendered_donation_statistics,
    ],
    REPORT_SECTION_SEPARATION,
  );

  tracing::info!("Generating chat message rankings.");
  let (unfiltered_chat_report, emote_filtered_chat_report) =
    get_messages_sent_ranking(query_conditions, None).await?;
  tracing::info!("Generating chat message rankings for subathon.");
  let (unfiltered_subathon_chat_report, subathon_emote_filtered_chat_report) =
    get_messages_sent_ranking(subathon_conditions, Some(SUBATHON_RANKING_ROW_LIMIT)).await?;

  tracing::info!("Gathering reports.");

  let reports = vec![
    general_stats_report,
    Report::new("unfiltered_chat_rankings", unfiltered_chat_report),
    Report::new("filtered_chat_rankings", emote_filtered_chat_report),
    Report::new(
      "unfiltered_subathon_chat_report",
      unfiltered_subathon_chat_report,
    ),
    Report::new(
      "subathon_emote_filtered_chat_report",
      subathon_emote_filtered_chat_report,
    ),
    general_stats_report_with_donations,
  ];

  Ok(reports)
}

/// Gets the list of reports that will only be added based on passed in flags or available data.
async fn get_conditional_reports(
  monthly_conditions: &AppQueryConditions,
  subathon_conditions: &AppQueryConditions,
  streamer_twitch_user_id: i32,
) -> Result<Vec<Report>, AppError> {
  tracing::info!("Generating conditional reports.");

  let mut conditional_reports = vec![];
  let subathon_start = subathon_conditions.date_start.unwrap();
  let subathon_end = subathon_conditions.date_end.unwrap_or(Utc::now());

  let donator_monthly_rankings_result = get_donation_rankings_for_streamer_and_date(
    streamer_twitch_user_id,
    subathon_start,
    subathon_end,
  )
  .await;

  match donator_monthly_rankings_result {
    Ok(donator_subathon_rankings) => conditional_reports.push(Report::new(
      "donator_subathon_rankings",
      donator_subathon_rankings,
    )),
    Err(error) => tracing::error!(
      "Failed to generate subathon donation rankings. Reason: {:?}",
      error
    ),
  }

  if Args::run_monthly_chat_ranking() {
    tracing::info!("Generating monthly chat message rankings.");

    let (monthly_unfiltered_chat_report, monthly_emote_filtered_chat_report) =
      get_messages_sent_ranking(monthly_conditions, Some(MONTHLY_RANKING_ROW_LIMIT)).await?;

    conditional_reports.push(Report::new(
      "monthly_unfiltered_chat_rankings",
      monthly_unfiltered_chat_report,
    ));
    conditional_reports.push(Report::new(
      "monthly_emote_filtered_chat_rankings",
      monthly_emote_filtered_chat_report,
    ));
  }

  Ok(conditional_reports)
}
