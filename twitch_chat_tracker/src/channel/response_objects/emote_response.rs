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
    for (service, emotes) in other.emotes {
      self
        .emotes
        .entry(service)
        .or_insert_with(Vec::new)
        .extend(emotes);
    }
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

#[cfg(test)]
mod tests {
  use super::*;
  use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};

  mod new {
    use super::*;

    #[test]
    fn creates_list_with_empty_vec_for_service() {
      let list = EmoteResponseList::new(ExternalService::SevenTv);

      assert!(list.emotes.contains_key(&ExternalService::SevenTv));
      assert!(list.emotes.get(&ExternalService::SevenTv).unwrap().is_empty());
    }
  }

  mod extend {
    use super::*;

    #[test]
    fn merges_different_services() {
      let mut list1 = EmoteResponseList {
        emotes: HashMap::from([(
          ExternalService::SevenTv,
          vec![EmoteResponse {
            id: "1".into(),
            name: "Emote1".into(),
          }],
        )]),
      };

      let list2 = EmoteResponseList {
        emotes: HashMap::from([(
          ExternalService::Bttv,
          vec![EmoteResponse {
            id: "2".into(),
            name: "Emote2".into(),
          }],
        )]),
      };

      list1.extend(list2);

      assert_eq!(list1.emotes.len(), 2);
      assert!(list1.emotes.contains_key(&ExternalService::SevenTv));
      assert!(list1.emotes.contains_key(&ExternalService::Bttv));
    }

    #[test]
    fn merges_same_service_emotes() {
      let mut list1 = EmoteResponseList {
        emotes: HashMap::from([(
          ExternalService::SevenTv,
          vec![EmoteResponse {
            id: "1".into(),
            name: "Original".into(),
          }],
        )]),
      };

      let list2 = EmoteResponseList {
        emotes: HashMap::from([(
          ExternalService::SevenTv,
          vec![EmoteResponse {
            id: "2".into(),
            name: "Additional".into(),
          }],
        )]),
      };

      list1.extend(list2);

      let emotes = list1.emotes.get(&ExternalService::SevenTv).unwrap();
      assert_eq!(emotes.len(), 2);
    }
  }

  mod build_emote_active_models_and_pairs {
    use super::*;

    #[test]
    fn builds_active_models_with_correct_fields() {
      let list = EmoteResponseList {
        emotes: HashMap::from([(
          ExternalService::SevenTv,
          vec![EmoteResponse {
            id: "ext_id_123".into(),
            name: "TestEmote".into(),
          }],
        )]),
      };

      let (active_models, _) = list.build_emote_active_models_and_pairs();

      assert_eq!(active_models.len(), 1);
      assert_eq!(active_models[0].external_id, Set("ext_id_123".into()));
      assert_eq!(active_models[0].name, Set("TestEmote".into()));
      assert_eq!(active_models[0].external_service, Set(ExternalService::SevenTv));
    }

    #[test]
    fn builds_pairs_with_external_id_and_service() {
      let list = EmoteResponseList {
        emotes: HashMap::from([(
          ExternalService::Bttv,
          vec![EmoteResponse {
            id: "bttv_id".into(),
            name: "BttvEmote".into(),
          }],
        )]),
      };

      let (_, pairs) = list.build_emote_active_models_and_pairs();

      assert_eq!(pairs.len(), 1);
      assert_eq!(pairs[0].0, "bttv_id");
      assert_eq!(pairs[0].1, ExternalService::Bttv);
    }

    #[test]
    fn handles_multiple_services() {
      let list = EmoteResponseList {
        emotes: HashMap::from([
          (
            ExternalService::SevenTv,
            vec![EmoteResponse {
              id: "7tv_1".into(),
              name: "SevenTvEmote".into(),
            }],
          ),
          (
            ExternalService::Bttv,
            vec![EmoteResponse {
              id: "bttv_1".into(),
              name: "BttvEmote".into(),
            }],
          ),
        ]),
      };

      let (active_models, pairs) = list.build_emote_active_models_and_pairs();

      assert_eq!(active_models.len(), 2);
      assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn returns_empty_for_empty_list() {
      let list = EmoteResponseList::default();

      let (active_models, pairs) = list.build_emote_active_models_and_pairs();

      assert!(active_models.is_empty());
      assert!(pairs.is_empty());
    }
  }

  mod batch_insert_emotes {
    use super::*;

    #[tokio::test]
    async fn inserts_and_retrieves_emotes() {
      let expected_emotes = vec![
        emote::Model {
          id: 1,
          external_id: "ext_1".into(),
          name: "Emote1".into(),
          external_service: ExternalService::SevenTv,
        },
        emote::Model {
          id: 2,
          external_id: "ext_2".into(),
          name: "Emote2".into(),
          external_service: ExternalService::SevenTv,
        },
      ];

      let database_connection = MockDatabase::new(DatabaseBackend::MySql)
        .append_exec_results([MockExecResult {
          last_insert_id: 1,
          rows_affected: 2,
        }])
        .append_query_results([expected_emotes.clone()])
        .into_connection();

      let list = EmoteResponseList {
        emotes: HashMap::from([(
          ExternalService::SevenTv,
          vec![
            EmoteResponse {
              id: "ext_1".into(),
              name: "Emote1".into(),
            },
            EmoteResponse {
              id: "ext_2".into(),
              name: "Emote2".into(),
            },
          ],
        )]),
      };

      let result = list.batch_insert_emotes(&database_connection).await;

      assert!(result.is_ok());
      let emotes = result.unwrap();
      assert_eq!(emotes.len(), 2);
      assert_eq!(emotes[0].external_id, "ext_1");
      assert_eq!(emotes[1].external_id, "ext_2");
    }

    #[tokio::test]
    async fn handles_multiple_services() {
      let expected_emotes = vec![
        emote::Model {
          id: 1,
          external_id: "7tv_1".into(),
          name: "SevenTvEmote".into(),
          external_service: ExternalService::SevenTv,
        },
        emote::Model {
          id: 2,
          external_id: "bttv_1".into(),
          name: "BttvEmote".into(),
          external_service: ExternalService::Bttv,
        },
      ];

      let database_connection = MockDatabase::new(DatabaseBackend::MySql)
        .append_exec_results([MockExecResult {
          last_insert_id: 1,
          rows_affected: 2,
        }])
        .append_query_results([expected_emotes.clone()])
        .into_connection();

      let list = EmoteResponseList {
        emotes: HashMap::from([
          (
            ExternalService::SevenTv,
            vec![EmoteResponse {
              id: "7tv_1".into(),
              name: "SevenTvEmote".into(),
            }],
          ),
          (
            ExternalService::Bttv,
            vec![EmoteResponse {
              id: "bttv_1".into(),
              name: "BttvEmote".into(),
            }],
          ),
        ]),
      };

      let result = list.batch_insert_emotes(&database_connection).await;

      assert!(result.is_ok());
      let emotes = result.unwrap();
      assert_eq!(emotes.len(), 2);
    }

    #[tokio::test]
    async fn returns_empty_vec_for_empty_list() {
      let database_connection = MockDatabase::new(DatabaseBackend::MySql)
        .append_exec_results([MockExecResult {
          last_insert_id: 0,
          rows_affected: 0,
        }])
        .append_query_results([Vec::<emote::Model>::new()])
        .into_connection();

      let list = EmoteResponseList::default();

      let result = list.batch_insert_emotes(&database_connection).await;

      assert!(result.is_ok());
      assert!(result.unwrap().is_empty());
    }
  }
}
