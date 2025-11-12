use entities::muted_vod_segment;
use sea_orm::{NotSet, Set};
use serde::Deserialize;

/// Based on Twitch's documentation here: https://dev.twitch.tv/docs/api/videos
#[derive(Deserialize, Debug)]
pub struct TwitchVodResponse {
  #[serde(rename = "data")]
  pub vod_list: Vec<TwitchVodData>,
}

#[derive(Deserialize, Debug)]
pub struct TwitchVodData {
  id: String,
  stream_id: Option<String>,

  #[allow(unused)]
  user_id: String,

  title: String,

  #[serde(default)]
  muted_segments: Option<Vec<MutedStreamSegment>>,
}

/// Values are in seconds.
#[derive(Deserialize, Debug, Clone)]
pub struct MutedStreamSegment {
  duration: i32,
  offset: i32,
}

impl TwitchVodData {
  pub fn vod_id(&self) -> &str {
    &self.id
  }

  /// Sometimes in really old vods the stream ID will be null.
  pub fn stream_id(&self) -> Option<&str> {
    self.stream_id.as_deref()
  }

  #[allow(unused)]
  pub fn user_twitch_id(&self) -> &str {
    &self.user_id
  }

  pub fn vod_title(&self) -> &str {
    &self.title
  }

  pub fn muted_segments(&self) -> &[MutedStreamSegment] {
    self.muted_segments.as_deref().unwrap_or(&[])
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

impl From<&MutedStreamSegment> for muted_vod_segment::ActiveModel {
  fn from(muted_segment: &MutedStreamSegment) -> Self {
    muted_vod_segment::ActiveModel {
      stream_id: NotSet,
      offset: Set(muted_segment.offset()),
      duration: Set(muted_segment.duration()),
    }
  }
}
