use crate::channel::tracked_channels::TrackedChannels;
use crate::errors::AppError;
use crate::processes::print_benchmarking;
use crate::processes::update_channel_livestreams::update_channel_live_streams;
use crate::processes::{app_animation::run_animation, process_irc_message_results};
use database_connection::get_database_connection;
use tokio::{sync::mpsc, task::JoinHandle};

/// Creates the necessary sub processes for running the app.
/// These include the running animation, channel updator, and message parsing result manager.
///
/// Returns the sender to the message parsing result manager.
pub async fn create_sub_processes() -> mpsc::UnboundedSender<JoinHandle<Result<bool, AppError>>> {
  tracing::info!("Creating sub processes.");
  let database_connection = get_database_connection().await;
  let connected_channels = TrackedChannels::new(database_connection).await.unwrap();
  let (irc_message_processing_sender, irc_message_processing_receiver) = mpsc::unbounded_channel();

  tokio::spawn(run_animation());
  tokio::spawn(update_channel_live_streams(connected_channels));
  tokio::spawn(process_irc_message_results(irc_message_processing_receiver));
  tokio::spawn(print_benchmarking());

  irc_message_processing_sender
}
