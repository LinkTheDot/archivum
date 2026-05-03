use crate::app::InterfaceConfig;
use crate::data_transfer_objects::chat_surge::ChatSurgeDto;
use crate::error::AppError;
use crate::response_models::{paginated_parameters::*, paginatied_response::*};
use crate::routes::helpers::get_stream::{get_stream, GetStream};
use crate::routes::helpers::serde::deserialize_from_string;
use axum::extract::{Query, State};
use entities::*;
use sea_orm::*;
use serde::Deserialize;
use std::collections::BTreeMap;

const MAX_PAGE_SIZE: u64 = 500;
const MIN_PAGE_SIZE: u64 = 1;
const DEFAULT_WINDOW_SIZE_SECONDS: u64 = 60;
const DEFAULT_SURGE_THRESHOLD: f64 = 2.0;
const DEFAULT_BASELINE_WINDOW_COUNT: usize = 30;

#[derive(Debug, Deserialize)]
pub struct ChatSurgesQuery {
  stream_id: Option<String>,
  twitch_stream_id: Option<String>,
  twitch_vod_id: Option<String>,

  #[serde(
    default = "default_window_size",
    deserialize_with = "deserialize_from_string"
  )]
  window_size_seconds: u64,

  #[serde(
    default = "default_surge_threshold",
    deserialize_with = "deserialize_from_string"
  )]
  surge_threshold: f64,

  /// How many preceding windows to average when computing the baseline
  /// for a given window. Defaults to 30.
  #[serde(
    default = "default_baseline_window_count",
    deserialize_with = "deserialize_from_string"
  )]
  baseline_window_count: usize,

  #[serde(flatten)]
  pagination_parameters: PaginationParameters,
}

fn default_window_size() -> u64 {
  DEFAULT_WINDOW_SIZE_SECONDS
}

fn default_surge_threshold() -> f64 {
  DEFAULT_SURGE_THRESHOLD
}

fn default_baseline_window_count() -> usize {
  DEFAULT_BASELINE_WINDOW_COUNT
}

impl GetStream for ChatSurgesQuery {
  fn get_stream_id(&self) -> Option<&str> {
    self.stream_id.as_deref()
  }

  fn get_twitch_stream_id(&self) -> Option<&str> {
    self.twitch_stream_id.as_deref()
  }

  fn get_twitch_vod_id(&self) -> Option<&str> {
    self.twitch_vod_id.as_deref()
  }
}

#[derive(Debug, serde::Serialize)]
pub struct ChatSurgesResponse {
  pub stream: stream::Model,
  pub window_size_seconds: u64,
  pub surge_threshold: f64,
  pub surges: Vec<ChatSurgeDto>,
}

#[axum::debug_handler]
pub async fn get_chat_surges(
  Query(query_payload): Query<ChatSurgesQuery>,
  State(interface_config): State<InterfaceConfig>,
) -> Result<axum::Json<PaginatedResponse<ChatSurgesResponse>>, AppError> {
  tracing::info!("Got a chat surges request: {query_payload:?}");

  let database_connection = interface_config.database_connection();
  let pagination = query_payload
    .pagination_parameters
    .clamped_page_size(MIN_PAGE_SIZE, MAX_PAGE_SIZE);

  let stream = get_stream(&query_payload, database_connection).await?;

  let stream_start = stream
    .start_timestamp
    .ok_or(AppError::StreamHasNoStartTimestamp {
      stream_id: stream.id,
    })?;

  let timestamps: Vec<chrono::DateTime<chrono::Utc>> = stream_message::Entity::find()
    .select_only()
    .column(stream_message::Column::Timestamp)
    .filter(stream_message::Column::StreamId.eq(stream.id))
    .order_by_asc(stream_message::Column::Timestamp)
    .into_tuple()
    .all(database_connection)
    .await?;

  let surges = compute_surges(
    &timestamps,
    stream_start,
    query_payload.window_size_seconds,
    query_payload.surge_threshold,
    query_payload.baseline_window_count,
  );

  let total_surges = surges.len() as u64;
  let total_pages = total_surges.div_ceil(pagination.page_size);
  let page_start = (pagination.page * pagination.page_size) as usize;
  let page_surges = surges
    .into_iter()
    .skip(page_start)
    .take(pagination.page_size as usize)
    .collect();

  Ok(axum::Json(PaginatedResponse {
    data: ChatSurgesResponse {
      stream,
      window_size_seconds: query_payload.window_size_seconds,
      surge_threshold: query_payload.surge_threshold,
      surges: page_surges,
    },
    pagination: Pagination {
      total_items: total_surges,
      total_pages,
      page: pagination.page,
      page_size: pagination.page_size,
    },
  }))
}

struct SurgeBucket {
  bucket: u64,
  count: u64,
  baseline: f64,
}

fn compute_surges(
  timestamps: &[chrono::DateTime<chrono::Utc>],
  stream_start: chrono::DateTime<chrono::Utc>,
  window_size_seconds: u64,
  surge_threshold: f64,
  baseline_window_count: usize,
) -> Vec<ChatSurgeDto> {
  if timestamps.is_empty() {
    return vec![];
  }

  let mut buckets: BTreeMap<u64, u64> = BTreeMap::new();
  for ts in timestamps {
    let elapsed = (*ts - stream_start).num_seconds().max(0) as u64;
    let bucket = elapsed / window_size_seconds;
    *buckets.entry(bucket).or_insert(0) += 1;
  }

  // BTreeMap iteration is sorted by key, so this preserves chronological order.
  let bucket_list: Vec<(u64, u64)> = buckets.into_iter().collect();

  let mut surge_buckets: Vec<SurgeBucket> = Vec::new();

  for (i, &(bucket, count)) in bucket_list.iter().enumerate() {
    // Need at least one preceding window to establish a local baseline.
    if i == 0 {
      continue;
    }

    let lookback_start = i.saturating_sub(baseline_window_count);
    let preceding = &bucket_list[lookback_start..i];
    let baseline =
      preceding.iter().map(|(_, c)| *c as f64).sum::<f64>() / preceding.len() as f64;

    if count as f64 >= baseline * surge_threshold {
      surge_buckets.push(SurgeBucket { bucket, count, baseline });
    }
  }

  // Merge consecutive surge buckets into a single surge entry.
  let mut surges: Vec<ChatSurgeDto> = Vec::new();
  let mut iter = surge_buckets.into_iter().peekable();

  while let Some(first) = iter.next() {
    let start_bucket = first.bucket;
    let mut last_bucket = first.bucket;
    let mut total_count = first.count;
    let mut baseline_sum = first.baseline;
    let mut merged_windows = 1u64;

    while iter.peek().is_some_and(|next| next.bucket == last_bucket + 1) {
      let next = iter.next().unwrap();
      last_bucket = next.bucket;
      total_count += next.count;
      baseline_sum += next.baseline;
      merged_windows += 1;
    }

    let avg_baseline = baseline_sum / merged_windows as f64;
    let avg_messages_per_window = total_count as f64 / merged_windows as f64;

    surges.push(ChatSurgeDto {
      window_start_seconds: (start_bucket * window_size_seconds) as i64,
      window_end_seconds: (last_bucket * window_size_seconds + window_size_seconds) as i64,
      message_count: total_count,
      baseline_message_count: avg_baseline,
      surge_factor: avg_messages_per_window / avg_baseline,
    });
  }

  surges
}
