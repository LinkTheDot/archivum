#[tokio::main]
async fn main() {
  nightly_checks::logging::setup_logging_config().unwrap();

  let process_succeeded = nightly_checks::checks::run().await;

  if !process_succeeded {
    std::process::exit(1)
  }
}
