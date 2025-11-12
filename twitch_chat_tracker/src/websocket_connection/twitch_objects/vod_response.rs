use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;

/// Based on Twitch's documentation here: https://dev.twitch.tv/docs/api/videos
#[derive(Deserialize, Debug)]
pub struct TwitchVodResponse {
  data: Vec<TwitchVodData>,
}

#[derive(Deserialize, Debug)]
pub struct TwitchVodData {
  id: String,
  stream_id: String,

  #[serde(deserialize_with = "deserialize_number_from_string")]
  user_id: i32,

  title: String,
  muted_segments: Vec<MutedStreamSegment>,
}

/// Values are in seconds.
#[derive(Deserialize, Debug, Clone)]
pub struct MutedStreamSegment {
  duration: i32,
  offset: i32,
}

impl TwitchVodResponse {
  pub fn get_vod_list(&self) -> &Vec<TwitchVodData> {
    &self.data
  }
}

impl TwitchVodData {
  pub fn vod_id(&self) -> &str {
    &self.id
  }

  pub fn stream_id(&self) -> &str {
    &self.stream_id
  }

  pub fn user_twitch_id(&self) -> i32 {
    self.user_id
  }

  pub fn vod_title(&self) -> &str {
    &self.title
  }

  pub fn muted_segments(&self) -> &Vec<MutedStreamSegment> {
    &self.muted_segments
  }
}

impl MutedStreamSegment {
  /// Returns the duration in seconds of the muted segment.
  pub fn duration(&self) -> i32 {
    self.duration
  }

  /// Returns the offset in seconds the muted segment was.
  pub fn offset(&self) -> i32 {
    self.offset
  }
}
