use crate::channel::response_objects::{
  bttv::emote_set::*,
  emote_response::{EmoteResponse, EmoteResponseList},
};
use entities::sea_orm_active_enums::ExternalService;
use std::collections::HashMap;

/// User API response: https://api.betterttv.net/3/cached/users/twitch/137524728
///
/// Last value in the URL is a user's Twitch ID.
#[derive(Debug, serde::Deserialize)]
pub struct BttvUserResponse {
  #[serde(rename = "sharedEmotes")]
  pub emote_set: Vec<BttvEmote>,
}

impl From<BttvUserResponse> for EmoteResponseList {
  fn from(global_response: BttvUserResponse) -> Self {
    let emotes: Vec<EmoteResponse> = global_response
      .emote_set
      .into_iter()
      .map(|emote_response| EmoteResponse {
        id: emote_response.id,
        name: emote_response.name,
      })
      .collect();

    EmoteResponseList {
      emotes: HashMap::from([(ExternalService::Bttv, emotes)]),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_user_response_with_shared_emotes() {
    let json = r#"{
      "sharedEmotes": [
        {"id": "user_bttv_1", "code": "UserBttvEmote1"},
        {"id": "user_bttv_2", "code": "UserBttvEmote2"}
      ]
    }"#;

    let response: BttvUserResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.emote_set.len(), 2);
    assert_eq!(response.emote_set[0].id, "user_bttv_1");
    assert_eq!(response.emote_set[0].name, "UserBttvEmote1");
  }

  #[test]
  fn parses_empty_shared_emotes() {
    let json = r#"{"sharedEmotes": []}"#;

    let response: BttvUserResponse = serde_json::from_str(json).unwrap();

    assert!(response.emote_set.is_empty());
  }

  #[test]
  fn converts_to_emote_response_list() {
    let json = r#"{
      "sharedEmotes": [
        {"id": "abc123", "code": "TestEmote"}
      ]
    }"#;

    let response: BttvUserResponse = serde_json::from_str(json).unwrap();
    let emote_list: EmoteResponseList = response.into();

    let emotes = emote_list.emotes.get(&ExternalService::Bttv).unwrap();
    assert_eq!(emotes.len(), 1);
    assert_eq!(emotes[0].id, "abc123");
    assert_eq!(emotes[0].name, "TestEmote");
  }

  #[test]
  fn missing_shared_emotes_field_returns_error() {
    let json = r#"{"otherField": []}"#;

    let result: Result<BttvUserResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn invalid_json_returns_error() {
    let json = r#"not valid json"#;

    let result: Result<BttvUserResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn wrong_emote_structure_returns_error() {
    let json = r#"{"sharedEmotes": [{"wrong": "structure"}]}"#;

    let result: Result<BttvUserResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }
}
