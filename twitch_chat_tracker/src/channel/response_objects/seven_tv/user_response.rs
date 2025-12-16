use crate::channel::response_objects::{
  emote_response::{EmoteResponse, EmoteResponseList},
  seven_tv::emote_response::SevenTvEmoteSet,
};
use entities::sea_orm_active_enums::ExternalService;
use serde_with::{serde_as, DefaultOnNull};
use std::collections::HashMap;

/// User API response: https://7tv.io/v3/users/twitch/578762718
///
/// Last value in the URL is a user's Twitch ID.
#[serde_as]
#[derive(Debug, serde::Deserialize)]
pub struct SevenTvUserResponse {
  #[serde_as(deserialize_as = "DefaultOnNull")]
  pub emote_set: SevenTvEmoteSet,
}

impl From<SevenTvUserResponse> for EmoteResponseList {
  fn from(global_response: SevenTvUserResponse) -> Self {
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
  fn parses_user_response_with_nested_emote_set() {
    let json = r#"{
      "emote_set": {
        "emotes": [
          {"id": "user_emote_1", "name": "UserEmote1"},
          {"id": "user_emote_2", "name": "UserEmote2"}
        ]
      }
    }"#;

    let response: SevenTvUserResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.emote_set.emotes.len(), 2);
    assert_eq!(response.emote_set.emotes[0].id, "user_emote_1");
    assert_eq!(response.emote_set.emotes[0].name, "UserEmote1");
  }

  #[test]
  fn parses_empty_emotes_array() {
    let json = r#"{"emote_set": {"emotes": []}}"#;

    let response: SevenTvUserResponse = serde_json::from_str(json).unwrap();

    assert!(response.emote_set.emotes.is_empty());
  }

  #[test]
  fn converts_to_emote_response_list() {
    let json = r#"{
      "emote_set": {
        "emotes": [
          {"id": "abc123", "name": "TestEmote"}
        ]
      }
    }"#;

    let response: SevenTvUserResponse = serde_json::from_str(json).unwrap();
    let emote_list: EmoteResponseList = response.into();

    let emotes = emote_list.emotes.get(&ExternalService::SevenTv).unwrap();
    assert_eq!(emotes.len(), 1);
    assert_eq!(emotes[0].id, "abc123");
    assert_eq!(emotes[0].name, "TestEmote");
  }

  #[test]
  fn missing_emote_set_field_returns_error() {
    let json = r#"{"other_field": "value"}"#;

    let result: Result<SevenTvUserResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn invalid_json_returns_error() {
    let json = r#"not valid json"#;

    let result: Result<SevenTvUserResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn wrong_emote_structure_returns_error() {
    let json = r#"{"emote_set": {"emotes": [{"wrong": "structure"}]}}"#;

    let result: Result<SevenTvUserResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }
}
