use crate::error::AppError;
use entities::*;
use entity_extensions::external_service::*;
use sea_orm::{DatabaseConnection, LoaderTrait, prelude::DateTimeUtc};

#[derive(Debug, serde::Serialize)]
pub struct StreamMessageDto {
  pub id: i32,
  pub is_first_message: bool,
  pub timestamp: DateTimeUtc,
  pub contents: String,
  pub is_subscriber: bool,
  /// Contents index and emote data.
  pub emote_usage: Vec<StreamMessageEmote>,
}

#[derive(Debug, serde::Serialize)]
pub struct StreamMessageEmote {
  pub contents_indices: Vec<usize>,
  pub emote_name_size: usize,
  pub emote_image_url: String,
}

impl StreamMessageDto {
  pub async fn convert_messages(
    user_messages: Vec<stream_message::Model>,
    database_connection: &DatabaseConnection,
    skip_emotes: bool,
  ) -> Result<Vec<Self>, AppError> {
    let emotes_used: Vec<Vec<emote::Model>> = if skip_emotes {
      vec![vec![]; user_messages.len()]
    } else {
      user_messages
        .load_many_to_many(emote::Entity, emote_usage::Entity, database_connection)
        .await?
    };

    Ok(
      user_messages
        .into_iter()
        .zip(emotes_used)
        .map(|(message, emotes)| {
          let message_contents = message.contents.unwrap_or_default();
          let mut emote_usage: Vec<StreamMessageEmote> = get_emote_usage(&message_contents, emotes);
          emote_usage.sort_by(|lhs, rhs| lhs.contents_indices.cmp(&rhs.contents_indices));

          StreamMessageDto {
            id: message.id,
            is_first_message: message.is_first_message != 0,
            timestamp: message.timestamp,
            contents: message_contents,
            is_subscriber: message.is_subscriber != 0,
            emote_usage,
          }
        })
        .collect(),
    )
  }
}

#[inline]
fn get_emote_usage(message_contents: &str, emotes: Vec<emote::Model>) -> Vec<StreamMessageEmote> {
  emotes
    .iter()
    .filter_map(|emote| {
      let mut index = 0;
      let emote_indices: Vec<usize> = message_contents
        // use split(' ') instead of split_whitespace() because we want to count
        // all spaces between any words. If there's two or more spaces this will account for them.
        .split(' ')
        .filter_map(|word| {
          let word_index = index;

          index += word.len() + 1;

          (word == emote.name).then_some(word_index)
        })
        .collect();

      if emote_indices.is_empty() {
        tracing::error!(
          "Failed to find emote {} in a message. Contents: {}",
          emote.id,
          message_contents
        );

        return None;
      }

      let emote_name_size = emote.name.len();
      let emote_fetch_url = emote.external_service.to_fetch_url(&emote.external_id);

      Some(StreamMessageEmote {
        contents_indices: emote_indices,
        emote_name_size,
        emote_image_url: emote_fetch_url.clone(),
      })
    })
    .collect()
}
