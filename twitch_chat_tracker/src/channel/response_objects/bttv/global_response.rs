use std::collections::HashMap;
use crate::channel::response_objects::{
  bttv::emote_set::*,
  emote_response::{EmoteResponse, EmoteResponseList},
};
use entities::sea_orm_active_enums::ExternalService;
use serde::{Deserialize, Deserializer};

/// Global API response: https://api.betterttv.net/3/cached/emotes/global
#[derive(Debug)]
pub struct BttvGlobalResponse {
  pub emotes: Vec<BttvEmote>,
}

impl<'de> Deserialize<'de> for BttvGlobalResponse {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let emotes = Vec::<BttvEmote>::deserialize(deserializer)?;

    Ok(BttvGlobalResponse { emotes })
  }
}

impl From<BttvGlobalResponse> for EmoteResponseList {
  fn from(global_response: BttvGlobalResponse) -> Self {
    let emotes: Vec<EmoteResponse> = global_response
      .emotes
      .into_iter()
      .map(|emote_response| EmoteResponse {
        id: emote_response.id,
        name: emote_response.name,
      })
      .collect();

    EmoteResponseList { emotes: HashMap::from([(ExternalService::Bttv, emotes)])  }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_array_response() {
    let json = r#"[
      {"id": "bttv_id_1", "code": "BttvEmote1"},
      {"id": "bttv_id_2", "code": "BttvEmote2"}
    ]"#;

    let response: BttvGlobalResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.emotes.len(), 2);
    assert_eq!(response.emotes[0].id, "bttv_id_1");
    assert_eq!(response.emotes[0].name, "BttvEmote1");
    assert_eq!(response.emotes[1].id, "bttv_id_2");
    assert_eq!(response.emotes[1].name, "BttvEmote2");
  }

  #[test]
  fn parses_empty_array() {
    let json = r#"[]"#;

    let response: BttvGlobalResponse = serde_json::from_str(json).unwrap();

    assert!(response.emotes.is_empty());
  }

  #[test]
  fn converts_to_emote_response_list() {
    let json = r#"[
      {"id": "abc123", "code": "TestEmote"},
      {"id": "def456", "code": "AnotherEmote"}
    ]"#;

    let response: BttvGlobalResponse = serde_json::from_str(json).unwrap();
    let emote_list: EmoteResponseList = response.into();

    let emotes = emote_list.emotes.get(&ExternalService::Bttv).unwrap();
    assert_eq!(emotes.len(), 2);
    assert_eq!(emotes[0].id, "abc123");
    assert_eq!(emotes[0].name, "TestEmote");
  }

  #[test]
  fn conversion_sets_correct_external_service() {
    let json = r#"[{"id": "test", "code": "Test"}]"#;

    let response: BttvGlobalResponse = serde_json::from_str(json).unwrap();
    let emote_list: EmoteResponseList = response.into();

    assert!(emote_list.emotes.contains_key(&ExternalService::Bttv));
    assert_eq!(emote_list.emotes.len(), 1);
  }

  #[test]
  fn object_instead_of_array_returns_error() {
    let json = r#"{"emotes": []}"#;

    let result: Result<BttvGlobalResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn invalid_json_returns_error() {
    let json = r#"not valid json"#;

    let result: Result<BttvGlobalResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn wrong_emote_structure_returns_error() {
    let json = r#"[{"wrong_field": "value"}]"#;

    let result: Result<BttvGlobalResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }
}
