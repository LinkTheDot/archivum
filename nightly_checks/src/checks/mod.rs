use crate::checks::{
  update_changed_names::DatabaseNameUpdateConfig, update_vod_data::config::UpdateVodDataConfig,
};

pub mod update_changed_names;
pub mod update_vod_data;

const REQUESTS_PER_MINUTE_LIMIT: usize = 10000;
const CHUNK_LIMIT: usize = 100;

const MAX_VOD_AGE_DAYS: usize = 30;

/// Returns true if the process finished successfully.
///
/// Returns false if any checks failed and exited out.
pub async fn run() -> bool {
  let mut process_succeeded = true;

  match DatabaseNameUpdateConfig::new(REQUESTS_PER_MINUTE_LIMIT, CHUNK_LIMIT).await {
    Ok(config) => config.run().await,
    Err(error) => {
      tracing::error!("Failed to create update name change config. Reason: `{error}`");
      process_succeeded = false;
    }
  }

  if let Err(error) = UpdateVodDataConfig::new(MAX_VOD_AGE_DAYS).await.run().await {
    tracing::error!("Failed to create update name change config. Reason: `{error}`");
    process_succeeded = false;
  }

  process_succeeded
}
