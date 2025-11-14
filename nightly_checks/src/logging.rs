use app_config::{log_level_wrapper::LoggingConfigLevel, AppConfig};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

const SEA_ORM_LOG_LEVEL: LoggingConfigLevel = LoggingConfigLevel::Warn;

pub fn setup_logging_config() -> Result<(), Box<dyn std::error::Error>> {
  let Some(log_level) = AppConfig::log_level() else {
    println!("Logging is disabled.");

    return Ok(());
  };

  let filter_string = format!(
    "{},sea_orm={seaorm_level},sea_orm_migration={seaorm_level},sqlx={seaorm_level}",
    log_level,
    seaorm_level = SEA_ORM_LOG_LEVEL
  );
  let env_filter = EnvFilter::new(filter_string);

  let subscriber_builder = tracing_subscriber::fmt()
    .with_env_filter(env_filter)
    .with_ansi(false);

  if let Some(logging_dir) = AppConfig::logging_dir() {
    println!("Logging to file");

    let filename_prefix = PathBuf::from(AppConfig::logging_filename_prefix());
    let logging_file = AppConfig::logging_file_roll_appender().clone();

    subscriber_builder
      .with_writer(logging_file.to_file_appender(logging_dir, &filename_prefix)?)
      .init();
  } else {
    println!("Logging to stdout.");

    subscriber_builder.init();
  }

  Ok(())
}
