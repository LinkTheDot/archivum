use crate::checks::update_vod_data::twitch_objects::vod_response::TwitchVodData;
use entities::{stream, twitch_user};

pub struct VodStreamPairs {
  pub _user: twitch_user::Model,
  pub vods_and_streams: Vec<VodAndStream>,
}

pub struct VodAndStream {
  pub vod: TwitchVodData,
  pub stream: stream::Model,
}
