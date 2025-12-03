use crate::errors::EntityExtensionError;
use entities::{emote_usage, stream_message};
use sea_orm::*;

pub trait EmoteUsageExtensions {
  /// Loads emote usage for a large number of stream messages in batches to avoid
  /// the "too many placeholders" error when the dataset is very large.
  ///
  /// This performs the same operation as `load_many` but breaks the query into
  /// chunks to stay within database placeholder limits.
  async fn load_many_batched(
    messages: &[stream_message::Model],
    database_connection: &DatabaseConnection,
    batch_size: usize,
  ) -> Result<Vec<Vec<emote_usage::Model>>, EntityExtensionError>;
}

impl EmoteUsageExtensions for emote_usage::Entity {
  async fn load_many_batched(
    messages: &[stream_message::Model],
    database_connection: &DatabaseConnection,
    batch_size: usize,
  ) -> Result<Vec<Vec<emote_usage::Model>>, EntityExtensionError> {
    let mut all_emote_usage: Vec<Vec<emote_usage::Model>> = Vec::new();

    for chunk in messages.chunks(batch_size) {
      let chunk_vec = chunk.to_vec();
      let emote_usage_batch = chunk_vec
        .load_many(emote_usage::Entity, database_connection)
        .await?;

      all_emote_usage.extend(emote_usage_batch);
    }

    Ok(all_emote_usage)
  }
}
