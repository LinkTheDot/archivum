use spanix_scrubber::clap::ClapArgs;
use spanix_scrubber::config::SpanixScrubberConfig;

#[tokio::main]
async fn main() {
  twitch_chat_tracker::logging::setup_logging_config().unwrap();

  let args = ClapArgs::new();
  let scrubber = SpanixScrubberConfig::new(&args.streamer_name).await;

  match () {
    _ if args.mode.scrub_data => scrubber.scrub_for_all_users_in_database_for_channel().await,
    _ if args.mode.process_data => {
      if let Some(data_set) = &args.data_set {
        scrubber.insert_user_messages_into_database(data_set).await;
      } else {
        tracing::error!("Attempted to process data without a given data set.");

        std::process::exit(1);
      }
    }
    _ => {
      unreachable!()
    }
  }
}
