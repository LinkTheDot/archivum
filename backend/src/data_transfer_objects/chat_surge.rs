/// Seconds elapsed since the stream's start timestamp.
#[derive(Debug, serde::Serialize)]
pub struct ChatSurgeDto {
  pub window_start_seconds: i64,
  pub window_end_seconds: i64,
  pub message_count: u64,
  pub baseline_message_count: f64,
  pub surge_factor: f64,
}
