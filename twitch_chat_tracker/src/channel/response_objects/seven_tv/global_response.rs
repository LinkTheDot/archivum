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

    EmoteResponseList { emotes }
  }
}
