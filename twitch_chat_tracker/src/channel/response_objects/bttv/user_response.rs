use crate::channel::response_objects::{
  bttv::emote_set::*,
  emote_response::{EmoteResponse, EmoteResponseList},
};

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

    EmoteResponseList { emotes }
  }
}
