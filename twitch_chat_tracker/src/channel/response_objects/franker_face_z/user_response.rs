use crate::channel::response_objects::{
  emote_response::{EmoteResponse, EmoteResponseList},
  franker_face_z::emote_response::*,
};
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

    EmoteResponseList { emotes }
  }
}
