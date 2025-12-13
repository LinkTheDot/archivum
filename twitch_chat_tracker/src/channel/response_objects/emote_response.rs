use crate::errors::AppError;
use entities::{emote, sea_orm_active_enums::ExternalService};
use sea_orm::*;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct EmoteResponseList {
  pub emotes: HashMap<ExternalService, Vec<EmoteResponse>>,
}

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

  pub fn extend(&mut self, other: Self) {
    self.emotes.extend(other.emotes);
  }

  pub async fn batch_insert_emotes(
    &self,
    database_connection: &DatabaseConnection,
  ) -> Result<(), AppError> {
    let emote_active_models: Vec<emote::ActiveModel> = self
      .emotes
      .iter()
      .flat_map(|(service_name, emote_list)| {
        emote_list
          .iter()
          .cloned()
          .map(|emote_response| emote::ActiveModel {
            external_service: Set(service_name.clone()),
            external_id: Set(emote_response.id),
            name: Set(emote_response.name),
            ..Default::default()
          })
          .collect::<Vec<emote::ActiveModel>>()
      })
      .collect();



    todo!("batch insert emotes");
  }
}
