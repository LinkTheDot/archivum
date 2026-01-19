use crate::channel::response_objects::{
  emote_response::{EmoteResponse, EmoteResponseList},
  franker_face_z::emote_response::*,
};
use entities::sea_orm_active_enums::ExternalService;
use std::collections::HashMap;

/// User API response: https://api.frankerfacez.com/v1/room/id/137524728
///
/// Last value in the URL is a user's Twitch ID.
#[derive(Debug, serde::Deserialize)]
pub struct FrankerFaceZUserResponse {
  #[serde(rename = "sets")]
  pub emote_set: HashMap<i32, FrankerFaceZUserEmoteSet>,
}

#[derive(Debug, serde::Deserialize)]
pub struct FrankerFaceZUserEmoteSet {
  pub emoticons: Vec<FrankerFaceZEmote>,
}

impl From<FrankerFaceZUserResponse> for EmoteResponseList {
  fn from(user_response: FrankerFaceZUserResponse) -> Self {
    let emotes: Vec<EmoteResponse> = user_response
      .emote_set
      .into_values()
      .flat_map(|emote_set| emote_set.emoticons)
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
  fn parses_user_response_with_sets() {
    let json = r#"{
      "sets": {
        "12345": {
          "emoticons": [
            {"id": 111, "name": "FFZUserEmote1"},
            {"id": 222, "name": "FFZUserEmote2"}
          ]
        }
      }
    }"#;

    let response: FrankerFaceZUserResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.emote_set.len(), 1);
    let emote_set = response.emote_set.get(&12345).unwrap();
    assert_eq!(emote_set.emoticons.len(), 2);
    assert_eq!(emote_set.emoticons[0].id, 111);
    assert_eq!(emote_set.emoticons[0].name, "FFZUserEmote1");
  }

  #[test]
  fn parses_multiple_sets() {
    let json = r#"{
      "sets": {
        "100": {
          "emoticons": [{"id": 1, "name": "Emote1"}]
        },
        "200": {
          "emoticons": [{"id": 2, "name": "Emote2"}]
        }
      }
    }"#;

    let response: FrankerFaceZUserResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.emote_set.len(), 2);
  }

  #[test]
  fn parses_empty_sets() {
    let json = r#"{"sets": {}}"#;

    let response: FrankerFaceZUserResponse = serde_json::from_str(json).unwrap();

    assert!(response.emote_set.is_empty());
  }

  #[test]
  fn converts_to_emote_response_list_flattening_sets() {
    let json = r#"{
      "sets": {
        "100": {
          "emoticons": [{"id": 1, "name": "Emote1"}]
        },
        "200": {
          "emoticons": [{"id": 2, "name": "Emote2"}, {"id": 3, "name": "Emote3"}]
        }
      }
    }"#;

    let response: FrankerFaceZUserResponse = serde_json::from_str(json).unwrap();
    let emote_list: EmoteResponseList = response.into();

    let emotes = emote_list
      .emotes
      .get(&ExternalService::FrankerFaceZ)
      .unwrap();
    assert_eq!(emotes.len(), 3);
  }

  #[test]
  fn conversion_converts_id_to_string() {
    let json = r#"{
      "sets": {
        "100": {
          "emoticons": [{"id": 12345, "name": "TestEmote"}]
        }
      }
    }"#;

    let response: FrankerFaceZUserResponse = serde_json::from_str(json).unwrap();
    let emote_list: EmoteResponseList = response.into();

    let emotes = emote_list
      .emotes
      .get(&ExternalService::FrankerFaceZ)
      .unwrap();
    assert_eq!(emotes[0].id, "12345");
  }

  #[test]
  fn missing_sets_field_returns_error() {
    let json = r#"{"other_field": {}}"#;

    let result: Result<FrankerFaceZUserResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn invalid_json_returns_error() {
    let json = r#"not valid json"#;

    let result: Result<FrankerFaceZUserResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn wrong_emoticons_structure_returns_error() {
    let json = r#"{"sets": {"100": {"emoticons": [{"wrong": "structure"}]}}}"#;

    let result: Result<FrankerFaceZUserResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }

  #[test]
  fn non_integer_set_key_returns_error() {
    let json = r#"{"sets": {"not_an_int": {"emoticons": []}}}"#;

    let result: Result<FrankerFaceZUserResponse, _> = serde_json::from_str(json);

    assert!(result.is_err());
  }
}
