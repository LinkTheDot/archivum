use crate::conditions::query_conditions_builder::AppQueryConditionsBuilder;
use crate::errors::AppError;
use crate::report_builders::tables::chat_messages::get_messages_sent_ranking;
use crate::reports::{Report, Reports};

const RANKING_ROW_LIMIT: usize = 1000;

pub async fn generate_reports(streamer_twitch_user_id: i32) -> Result<Reports, AppError> {
  let mut reports = Reports::default();

  let message_conditions = AppQueryConditionsBuilder::new()
    .set_streamer_twitch_user_id(streamer_twitch_user_id)
    .build()?;

  tracing::info!("Generating message_rankings for overall messages for channel {{ id: {streamer_twitch_user_id} }}.");

  let (unfiltered_chat_report, quality_filtered_chat_report) =
    get_messages_sent_ranking(&message_conditions, Some(RANKING_ROW_LIMIT)).await?;

  let message_reports = vec![
    Report::new("unfiltered_overall_chat_report", unfiltered_chat_report),
    Report::new("overall_filtered_chat_report", quality_filtered_chat_report),
  ];

  reports.add_reports(message_reports);

  Ok(reports)
}
