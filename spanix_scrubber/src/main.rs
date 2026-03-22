use spanix_scrubber::{clap::ClapArgs, config::SpanixScrubberConfig};

#[tokio::main]
async fn main() {
  twitch_chat_tracker::logging::setup_logging_config().unwrap();

  old().await;
}

async fn old() {
  let args = ClapArgs::new();
  let scrubber = SpanixScrubberConfig::new(&args.streamer_name)
    .await
    .unwrap();

  // use entities::*;
  // use sea_orm::*;
  // use spanix_scrubber::user_chat_months::*;
  // use sea_orm::prelude::Expr;
  // let rows = stream_message::Entity::find()
  //   .select_only()
  //   .column(stream_message::Column::TwitchUserId)
  //   .column_as(Expr::cust("YEAR(timestamp)"), "chat_year")
  //   .column_as(Expr::cust("MONTH(timestamp)"), "chat_month")
  //   .filter(stream_message::Column::ChannelId.eq(1))
  //   .group_by(stream_message::Column::TwitchUserId)
  //   .group_by(Expr::cust("YEAR(timestamp)"))
  //   .group_by(Expr::cust("MONTH(timestamp)"))
  //   .build(DbBackend::MySql);
  // println!("{}", rows.to_string());
  // return;

  scrubber.scrape_spanix_messages().await;

  // match () {
  //   _ if args.mode.scrub_data => scrubber.scrub_for_all_users_in_database_for_channel().await,
  //   _ if args.mode.process_data => {
  //     if let Some(data_set) = &args.data_set {
  //       scrubber.insert_user_messages_into_database(data_set).await;
  //     } else {
  //       tracing::error!("Attempted to process data without a given data set.");
  //
  //       std::process::exit(1);
  //     }
  //   }
  //   _ => {
  //     unreachable!()
  //   }
  // }
}
