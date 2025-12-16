use crate::channel::response_objects::{
  emote_response::{EmoteResponse, EmoteResponseList},
  franker_face_z::emote_response::*,
};
use entities::sea_orm_active_enums::ExternalService;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

/// Global API response: https://api.betterttv.net/3/cached/frankerfacez/emotes/global
#[derive(Debug)]
pub struct FrankerFaceZGlobalResponse {
  pub emotes: Vec<FrankerFaceZEmote>,
}

impl<'de> Deserialize<'de> for FrankerFaceZGlobalResponse {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let emotes = Vec::<FrankerFaceZEmote>::deserialize(deserializer)?;

    Ok(FrankerFaceZGlobalResponse { emotes })
  }
}

impl From<FrankerFaceZGlobalResponse> for EmoteResponseList {
  fn from(global_response: FrankerFaceZGlobalResponse) -> Self {
    let emotes: Vec<EmoteResponse> = global_response
      .emotes
      .into_iter()
      .map(|emote_response| EmoteResponse {
        id: emote_response.id.to_string(),
        name: emote_response.name,
      })
      .collect();

    EmoteResponseList {
      emotes: HashMap::from([(ExternalService::FrankerFaceZ, emotes)]),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_array_response() {
    let json = r#"[
      {"id": 12345, "code": "FFZEmote1"},
      {"id": 67890, "code": "FFZEmote2"}
    ]"#;

    let response: FrankerFaceZGlobalResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.emotes.len(), 2);
    assert_eq!(response.emotes[0].id, 12345);
    assert_eq!(response.emotes[0].name, "FFZEmote1");
    assert_eq!(response.emotes[1].id, 67890);
    assert_eq!(response.emotes[1].name, "FFZEmote2");
  }

  #[test]
  fn parses_empty_array() {
    let json = r#"[]"#;

    let response: FrankerFaceZGlobalResponse = serde_json::from_str(json).unwrap();

    assert!(response.emotes.is_empty());
  }

  #[test]
  fn converts_to_emote_response_list_with_string_id() {
    let json = r#"[
      {"id": 123, "code": "TestEmote"},
      {"id": 456, "code": "AnotherEmote"}
    ]"#;

    let response: FrankerFaceZGlobalResponse = serde_json::from_str(json).unwrap();
    let emote_list: EmoteResponseList = response.into();

    let emotes = emote_list.emotes.get(&ExternalService::FrankerFaceZ).unwrap();
    assert_eq!(emotes.len(), 2);
    assert_eq!(emotes[0].id, "123");
    assert_eq!(emotes[0].name, "TestEmote");
    assert_eq!(emotes[1].id, "456");
  }

  #[test]
  fn conversion_sets_correct_external_service() {
    let json = r#"[{"id": 1, "code": "Test"}]"#;

    let response: FrankerFaceZGlobalResponse = serde_json::from_str(json).unwrap();
    let emote_list: EmoteResponseList = response.into();

    assert!(emote_list.emotes.contains_key(&ExternalService::FrankerFaceZ));
    assert_eq!(emote_list.emotes.len(), 1);
  }

  #[test]
  fn object_instead_of_array_returns_error() {
    let json = r#"{"emotes": []}"#;

    let result: Result<FrankerFaceZGlobalResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn invalid_json_returns_error() {
    let json = r#"not valid json"#;

    let result: Result<FrankerFaceZGlobalResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn wrong_emote_structure_returns_error() {
    let json = r#"[{"wrong_field": "value"}]"#;

    let result: Result<FrankerFaceZGlobalResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn string_id_instead_of_integer_returns_error() {
    let json = r#"[{"id": "not_an_int", "code": "Test"}]"#;

    let result: Result<FrankerFaceZGlobalResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }
}
