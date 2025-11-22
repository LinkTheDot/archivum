use chrono::{DateTime, Utc};

#[derive(Debug, serde::Deserialize)]
pub(super) struct TwitchStreamResponse {
  pub data: Vec<TwitchStreamData>,
}

#[derive(Debug, serde::Deserialize)]
pub struct TwitchStreamData {
  #[serde(rename = "id")]
  pub stream_id: String,
  pub user_id: String,
  pub user_login: String,
  #[serde(rename = "type")]
  pub stream_status_type: String,
  pub title: String,
  pub started_at: DateTime<Utc>,
}
