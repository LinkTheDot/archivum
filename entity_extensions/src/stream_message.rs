use crate::errors::EntityExtensionError;
use entities::*;
use sea_orm::*;
use sea_query::OnConflict;

pub trait StreamMessageExtensions {
  async fn insert_many_emote_usages(
    emote_usage_active_models: Vec<emote_usage::ActiveModel>,
    database_connection: &DatabaseConnection,
  ) -> Result<(), EntityExtensionError>;

  async fn chunked_related_emotes(
    messages: &Vec<Self>,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<Vec<emote::Model>>, EntityExtensionError>
  where
    Self: Sized;
}

impl StreamMessageExtensions for stream_message::Model {
  async fn insert_many_emote_usages(
    emote_usage_active_models: Vec<emote_usage::ActiveModel>,
    database_connection: &DatabaseConnection,
  ) -> Result<(), EntityExtensionError> {
    let potentional_conflicting_columns = [
      emote_usage::Column::EmoteId,
      emote_usage::Column::StreamMessageId,
    ];

    emote_usage::Entity::insert_many(emote_usage_active_models)
      .on_conflict(
        OnConflict::columns(potentional_conflicting_columns)
          .do_nothing_on(potentional_conflicting_columns)
          .to_owned(),
      )
      .do_nothing()
      .exec(database_connection)
      .await?;

    Ok(())
  }

  async fn chunked_related_emotes(
    messages: &Vec<Self>,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<Vec<emote::Model>>, EntityExtensionError>
  where
    Self: Sized,
  {
    let mut final_joined_emote_list: Vec<Vec<emote::Model>> = vec![];

    for messages_chunk in messages.chunks(u16::MAX as usize) {
      let mut many_emotes: Vec<Vec<emote::Model>> = messages_chunk
        .load_many_to_many(emote::Entity, emote_usage::Entity, database_connection)
        .await?;

      final_joined_emote_list.append(&mut many_emotes);
    }

    Ok(final_joined_emote_list)
  }
}
