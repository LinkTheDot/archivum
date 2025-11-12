use std::time::{Duration, Instant};

const RATELIMIT_DURATION: Duration = Duration::new(60, 0);

pub struct RateLimiter {
  rate_per_minute: usize,
  used_since_last_refresh: usize,
  last_refresh: Instant,
}

impl RateLimiter {
  pub fn new(rate_per_minute: usize) -> Self {
    Self {
      rate_per_minute,
      used_since_last_refresh: 0,
      last_refresh: Instant::now(),
    }
  }

  /// Requests the given amount of tokens. If the total tokens requested since the last refresh have been exceded, the duration until next refresh is returned.
  pub fn request_tokens(&mut self, tokens: usize) -> Option<Duration> {
    let last_refresh = self.last_refresh.elapsed();

    if last_refresh >= RATELIMIT_DURATION {
      self.refresh();

      self.used_since_last_refresh += tokens;

      return None;
    } else if self.used_since_last_refresh + tokens >= self.rate_per_minute {
      return Some(RATELIMIT_DURATION - last_refresh);
    }

    self.used_since_last_refresh += tokens;

    None
  }

  fn refresh(&mut self) {
    self.last_refresh = Instant::now();
  }

  /// Called when the full reset duration has been waited to reset the available tokens.
  pub fn reset_tokens(&mut self) {
    self.used_since_last_refresh = 0;
  }

  pub fn tokens(&self) -> usize {
    self
      .rate_per_minute
      .saturating_sub(self.used_since_last_refresh)
  }
}
