use database_connection::*;
use entities::{stream, twitch_user};
use entity_extensions::twitch_user::*;
use report_generator::clap::Args;
use report_generator::conditions::query_conditions_builder::AppQueryConditionsBuilder;
use report_generator::reports::chosen_report::ChosenReport;
use report_generator::reports::subathon_points::get_points_for_subathon;
use report_generator::reports::*;
use report_generator::upload_reports::upload_reports;
use sea_orm::*;

#[tokio::main]
async fn main() {
  report_generator::logging::setup_logging_config().unwrap();

  let database_connection = get_database_connection().await;
  let stream = get_stream(database_connection).await;

  let condition = AppQueryConditionsBuilder::new()
    .set_stream_id(stream.id)
    .set_streamer_twitch_user_id(stream.twitch_user_id)
    .build()
    .unwrap();

  let generate_reports_result = match Args::chosen_report() {
    ChosenReport::Basic => basic_reports::generate_reports(condition, stream.twitch_user_id).await,
    ChosenReport::Subathon => {
      subathon_reports::generate_reports(condition, stream.twitch_user_id).await
    }
    ChosenReport::CalculateSubathonPoints => {
      let points = get_points_for_subathon(stream.twitch_user_id)
        .await
        .unwrap();

      println!("Total points for subathon: `{points}`");

      std::process::exit(0);
    }
    ChosenReport::AnnualMessages => annual_chats::generate_reports(stream.twitch_user_id).await,
  };

  match generate_reports_result {
    Ok(reports) => {
      println!("\n\n");

      if let Err(error) = upload_reports(stream, reports).await {
        tracing::error!("Failed to upload the reports. Reason: {:?}", error);
      }
    }
    Err(error) => {
      tracing::error!("Failed to generate a report. Reason: {:?}", error);
    }
  }
}

/// Returns the latest stream for the streamer based on arguments given to the program.
///
/// The stream id will take priority, then a streamer name will be checked.
///
/// The program will exit if no stream was found.
async fn get_stream(database_connection: &DatabaseConnection) -> stream::Model {
  if let Some(stream_id) = Args::report_stream_id() {
    let stream = stream::Entity::find_by_id(stream_id)
      .one(database_connection)
      .await
      .unwrap()
      .unwrap();

    return stream;
  }

  if let Some(streamer_name) = Args::streamer_name_report() {
    let streamer_twitch_user_model =
      twitch_user::Model::get_or_set_by_name(streamer_name, database_connection)
        .await
        .unwrap();
    let maybe_latest_stream = stream::Entity::find()
      .filter(stream::Column::TwitchUserId.eq(streamer_twitch_user_model.id))
      .order_by_desc(stream::Column::Id)
      .one(database_connection)
      .await
      .unwrap();

    if let Some(latest_stream) = maybe_latest_stream {
      return latest_stream;
    }
  }

  tracing::error!("Failed to find a user or stream to generate reports for.");

  std::process::exit(1);
}
