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
