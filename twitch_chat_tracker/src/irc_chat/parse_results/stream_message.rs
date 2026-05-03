use crate::{channel::third_party_emote_list_storage::EmoteListStorage, errors::AppError};
use entities::*;
use entity_extensions::emote::EmoteExtensions;
use sea_orm::*;
use sea_orm_active_enums::ExternalService;
use std::{collections::HashMap, marker::PhantomData};

#[derive(Debug)]
pub struct ParsedStreamMessage<'a, ModelState = ActiveModel> {
  /// The list is taken as Twitch's IRC client provides.
  /// Formatted as: `emote_id:0-1,2-3/`
  ///
  /// Returned as a list of the emote models and the positions they were used in the message.
  pub twitch_emote_data: &'a str,
  pub channel: twitch_user::Model,
  stream_message_model: StoredMessageModel,

  model_state: PhantomData<ModelState>,
}

#[derive(Debug)]
pub struct ActiveModel;
#[derive(Debug)]
pub struct Model;

#[derive(Debug)]
enum StoredMessageModel {
  ActiveModel(stream_message::ActiveModel),
  Model(stream_message::Model),
}

#[derive(Debug)]
struct ParsedTwitchEmote {
  emote_active_model: emote::ActiveModel,
  usage_count: i32,
}

impl<'a> ParsedStreamMessage<'a, ActiveModel> {
  pub fn new(
    stream_message_active_model: stream_message::ActiveModel,
    twitch_emote_data: &'a str,
    channel: twitch_user::Model,
  ) -> Self {
    Self {
      stream_message_model: StoredMessageModel::ActiveModel(stream_message_active_model),
      twitch_emote_data,
      channel,

      model_state: PhantomData,
    }
  }

  pub async fn insert_message(
    mut self,
    database_connection: &DatabaseConnection,
  ) -> Result<ParsedStreamMessage<'a, Model>, AppError> {
    if let StoredMessageModel::ActiveModel(stream_message_active_model) = self.stream_message_model
    {
      let resulting_model = stream_message_active_model
        .insert(database_connection)
        .await?;

      self.stream_message_model = StoredMessageModel::Model(resulting_model);
    }

    Ok(ParsedStreamMessage {
      stream_message_model: self.stream_message_model,
      twitch_emote_data: self.twitch_emote_data,
      channel: self.channel,
      model_state: PhantomData,
    })
  }
}

impl ParsedStreamMessage<'_, Model> {
  pub async fn parse_emote_usage(
    &self,
    third_party_emote_list_storage: &EmoteListStorage,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<emote_usage::ActiveModel>, AppError> {
    let mut emote_usage_active_models =
      self.parse_7tv_emote_usage_from_message(third_party_emote_list_storage);
    let twitch_emote_usage_active_models = self
      .parse_twitch_emote_usage_from_message(database_connection)
      .await?;

    emote_usage_active_models.extend(twitch_emote_usage_active_models);

    Ok(emote_usage_active_models)
  }

  fn parse_7tv_emote_usage_from_message(
    &self,
    third_party_emote_list_storage: &EmoteListStorage,
  ) -> Vec<emote_usage::ActiveModel> {
    let StoredMessageModel::Model(stream_message) = &self.stream_message_model else {
      tracing::error!(
        "Unreachable broken state has been reached when parsing a stream message. Message dump: {:#?}",
        self
      );

      return vec![];
    };
    let Some(message_contents) = &stream_message.contents else {
      return vec![];
    };
    let emote_usage: HashMap<i32, i32> = message_contents
      .split(' ')
      .filter_map(|word| third_party_emote_list_storage.get_channel_emote(&self.channel, word))
      .fold(HashMap::new(), |mut emote_and_frequency, emote| {
        let entry = emote_and_frequency.entry(emote.id).or_default();
        *entry += 1;

        emote_and_frequency
      });

    let mut end_emote_usage_list = vec![];

    for (emote_id, usage_count) in emote_usage {
      let emote_usage_active_model = emote_usage::ActiveModel {
        stream_message_id: Set(stream_message.id),
        emote_id: Set(emote_id),
        usage_count: Set(usage_count),
      };

      end_emote_usage_list.push(emote_usage_active_model);
    }

    end_emote_usage_list
  }

  async fn parse_twitch_emote_usage_from_message(
    &self,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<emote_usage::ActiveModel>, AppError> {
    let StoredMessageModel::Model(stream_message) = &self.stream_message_model else {
      tracing::error!(
        "Unreachable broken state has been reached when parsing a stream message. Message dump: {:#?}",
        self
      );

      return Ok(vec![]);
    };
    let Some(message_contents) = &stream_message.contents else {
      return Ok(vec![]);
    };

    let emote_usage_data = parse_twitch_emotes(message_contents, self.twitch_emote_data);
    let mut parsed_emotes = vec![];

    for ParsedTwitchEmote {
      emote_active_model,
      usage_count,
    } in emote_usage_data
    {
      let emote =
        emote::Model::get_or_set_active_model(emote_active_model, database_connection).await?;

      let emote_usage = emote_usage::ActiveModel {
        stream_message_id: Set(stream_message.id),
        emote_id: Set(emote.id),
        usage_count: Set(usage_count),
      };

      parsed_emotes.push(emote_usage);
    }

    Ok(parsed_emotes)
  }
}

// let third_party_emotes_used =
//   self.parse_7tv_emotes_from_message_contents(&streamer_twitch_user_model, message_contents);
// let emote_list =
//   emote::Model::get_or_set_twitch_list(message_contents, emotes, database_connection).await?;

/// The list is formatted as: `emote_id:0-1,2-3/` as per Twitch's emote storage.
fn parse_twitch_emotes(
  message_contents: &str,
  twitch_emote_list_response: &str,
) -> Vec<ParsedTwitchEmote> {
  let emote_list = twitch_emote_list_response.split('/');

  emote_list.filter_map(|emote_usage| {
    let mut emote_usage = emote_usage.split(':');
    let emote_id = emote_usage.next()?;
    let emote_positions: Vec<(usize, usize)> = emote_usage
      .next()?
      .split(',')
      .filter_map(|emote_positions| {
        let mut positions = emote_positions.split('-');
        let start = positions.next()?.parse::<usize>().ok()?;
        let end = positions.next()?.parse::<usize>().ok()?;

        Some((start, end))
      })
      .collect();
    let (emote_name_start, emote_name_end) = *emote_positions.first()?;

    if message_contents.chars().count() < emote_name_end {
      tracing::error!("Encountered a message where the emote position provided is larger than the message length. Message contents: {:?} | Emote range: {}-{}", message_contents, emote_name_start, emote_name_end);
      return None;
    }

    let emote_name: String = message_contents
      .chars()
      .skip(emote_name_start)
      .take(emote_name_end - emote_name_start + 1)
      .collect();

    let emote_active_model = emote::ActiveModel {
      external_id: Set(emote_id.to_owned()),
      name: Set(emote_name),
      external_service: Set(ExternalService::Twitch),
      ..Default::default()
    };

    Some(ParsedTwitchEmote {
      emote_active_model,
      usage_count: emote_positions.len() as i32,
    })
  }).collect()
}
