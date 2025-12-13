use crate::channel::response_objects::{
  emote_response::{EmoteResponse, EmoteResponseList},
  seven_tv::emote_response::SevenTvEmoteSet,
};

/// User API response: https://7tv.io/v3/users/twitch/578762718
///
/// Last value in the URL is a user's Twitch ID.
#[derive(Debug, serde::Deserialize)]
pub struct SevenTvUserResponse {
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

    EmoteResponseList { emotes }
  }
}
