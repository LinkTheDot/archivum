use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct ActiveStreams {
  stream_twitch_ids: HashSet<String>,
}

impl ActiveStreams {
  /// Sets the list of active streams to the given list of `twitch stream ids`.
  ///
  /// The value MUST be Twitch's own stream id.
  pub fn add_stream(&mut self, stream_twitch_id: &str) {
    self.stream_twitch_ids.insert(stream_twitch_id.to_string());
  }

  pub fn contains_stream(&self, stream_twitch_id: &str) -> bool {
    self.stream_twitch_ids.contains(stream_twitch_id)
  }

  pub fn streams<I>(&self) -> impl IntoIterator<Item = &String> {
    &self.stream_twitch_ids
  }

  /// Removes streams from the list if they meet a given condition. Returns the list of removed stream Twitch ids.
  ///
  /// Takes a closure that passes in the list of every active stream to determine if it should be removed or not.
  pub fn extract_if(&mut self, condition: impl Fn(&String) -> bool) -> Vec<String> {
    self.stream_twitch_ids.extract_if(condition).collect()
  }
}
