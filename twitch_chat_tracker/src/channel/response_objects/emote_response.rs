use crate::errors::AppError;
use entities::{emote, sea_orm_active_enums::ExternalService};
use sea_orm::{prelude::Expr, sea_query::OnConflict, *};
use std::collections::HashMap;

/// The built list of emote responses from different external services.
#[derive(Debug, Default)]
pub struct EmoteResponseList {
  pub emotes: HashMap<ExternalService, Vec<EmoteResponse>>,
}

/// A basic object to define the required fields that makes an emote.
#[derive(Debug, Clone)]
pub struct EmoteResponse {
  pub id: String,
  pub name: String,
}

impl EmoteResponseList {
  pub fn new(external_service: ExternalService) -> Self {
    Self {
      emotes: HashMap::from([(external_service, vec![])]),
    }
  }

  /// Extends the internal list of emotes using another instance of emote responses.
  ///
  /// Merges lists for duplicate external services stored.
  pub fn extend(&mut self, other: Self) {
    self.emotes.extend(other.emotes);
  }

  /// Attempts to insert the emotes stored into the database, ignoring conflicts.
  ///
  /// Returns the models for stored emotes.
  pub async fn batch_insert_emotes(
    self,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<emote::Model>, AppError> {
    let (emote_active_models, emote_external_id_service_pairs) =
      self.build_emote_active_models_and_pairs();

    let potentional_conflicting_columns =
      [emote::Column::ExternalService, emote::Column::ExternalId];

    emote::Entity::insert_many(emote_active_models)
      .on_conflict(
        OnConflict::columns(potentional_conflicting_columns)
          .do_nothing_on(potentional_conflicting_columns)
          .to_owned(),
      )
      .do_nothing()
      .exec(database_connection)
      .await?;

    let emotes = emote::Entity::find()
      .filter(
        Expr::tuple([
          Expr::col(emote::Column::ExternalId).into(),
          Expr::col(emote::Column::ExternalService).into(),
        ])
        .in_tuples(emote_external_id_service_pairs),
      )
      .all(database_connection)
      .await?;

    Ok(emotes)
  }

  /// Builds the lists of emote active models for insertion and
  /// (external_id, external_service) data for retrieval
  fn build_emote_active_models_and_pairs(
    &self,
  ) -> (Vec<emote::ActiveModel>, Vec<(String, ExternalService)>) {
    self
      .emotes
      .iter()
      .flat_map(|(service_name, emote_list)| {
        emote_list
          .iter()
          .cloned()
          .map(|emote_response| {
            let active_model = emote::ActiveModel {
              external_service: Set(service_name.clone()),
              external_id: Set(emote_response.id.clone()),
              name: Set(emote_response.name.clone()),
              ..Default::default()
            };

            let pair = (emote_response.id, service_name.clone());

            (active_model, pair)
          })
          .collect::<Vec<(emote::ActiveModel, (String, ExternalService))>>()
      })
      .unzip()
  }
}
