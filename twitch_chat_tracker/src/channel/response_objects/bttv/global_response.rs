use crate::channel::response_objects::{
  bttv::emote_set::*,
  emote_response::{EmoteResponse, EmoteResponseList},
};
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

    EmoteResponseList { emotes }
  }
}
