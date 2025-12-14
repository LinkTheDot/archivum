use std::collections::HashMap;
use entities::sea_orm_active_enums::ExternalService;
use crate::channel::response_objects::{
  emote_response::{EmoteResponse, EmoteResponseList},
  seven_tv::emote_response::*,
};

/// Global API response: https://7tv.io/v3/emote-sets/global
#[derive(Debug, serde::Deserialize)]
pub struct SevenTvGlobalResponse {
  #[serde(flatten)]
  pub emote_set: SevenTvEmoteSet,
}

impl From<SevenTvGlobalResponse> for EmoteResponseList {
  fn from(global_response: SevenTvGlobalResponse) -> Self {
    let emotes: Vec<EmoteResponse> = global_response
      .emote_set
      .emotes
      .into_iter()
      .map(|emote_response| EmoteResponse {
        id: emote_response.id,
        name: emote_response.name,
      })
      .collect();

    EmoteResponseList {
      emotes: HashMap::from([(ExternalService::SevenTv, emotes)]),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_global_response_with_emotes() {
    let json = r#"{
      "emotes": [
        {"id": "60ae958e229664e8667aea38", "name": "Sadge"},
        {"id": "60ae3e54259ac5a73e56a426", "name": "KEKW"}
      ]
    }"#;

    let response: SevenTvGlobalResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.emote_set.emotes.len(), 2);
    assert_eq!(response.emote_set.emotes[0].id, "60ae958e229664e8667aea38");
    assert_eq!(response.emote_set.emotes[0].name, "Sadge");
    assert_eq!(response.emote_set.emotes[1].name, "KEKW");
  }

  #[test]
  fn parses_empty_emotes_array() {
    let json = r#"{"emotes": []}"#;

    let response: SevenTvGlobalResponse = serde_json::from_str(json).unwrap();

    assert!(response.emote_set.emotes.is_empty());
  }

  #[test]
  fn converts_to_emote_response_list() {
    let json = r#"{
      "emotes": [
        {"id": "abc123", "name": "TestEmote"},
        {"id": "def456", "name": "AnotherEmote"}
      ]
    }"#;

    let response: SevenTvGlobalResponse = serde_json::from_str(json).unwrap();
    let emote_list: EmoteResponseList = response.into();

    let emotes = emote_list.emotes.get(&ExternalService::SevenTv).unwrap();
    assert_eq!(emotes.len(), 2);
    assert_eq!(emotes[0].id, "abc123");
    assert_eq!(emotes[0].name, "TestEmote");
    assert_eq!(emotes[1].id, "def456");
    assert_eq!(emotes[1].name, "AnotherEmote");
  }

  #[test]
  fn conversion_sets_correct_external_service() {
    let json = r#"{"emotes": [{"id": "test", "name": "Test"}]}"#;

    let response: SevenTvGlobalResponse = serde_json::from_str(json).unwrap();
    let emote_list: EmoteResponseList = response.into();

    assert!(emote_list.emotes.contains_key(&ExternalService::SevenTv));
    assert_eq!(emote_list.emotes.len(), 1);
  }

  #[test]
  fn missing_emotes_field_returns_error() {
    let json = r#"{"other_field": "value"}"#;

    let result: Result<SevenTvGlobalResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn invalid_json_returns_error() {
    let json = r#"not valid json"#;

    let result: Result<SevenTvGlobalResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn wrong_emote_structure_returns_error() {
    let json = r#"{"emotes": [{"wrong_field": "value"}]}"#;

    let result: Result<SevenTvGlobalResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }
}
