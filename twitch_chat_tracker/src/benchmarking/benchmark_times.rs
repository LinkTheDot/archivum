use std::time::Duration;

#[derive(Debug, Default, Clone)]
pub struct BenchmarkTimes {
  added_times: usize,
  total_time: Duration,
}

impl BenchmarkTimes {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn add_time(&mut self, added_time: Duration) {
    self.added_times += 1;
    self.total_time += added_time;
  }

  pub fn average_times(&self) -> Duration {
    if self.added_times == 0 {
      Duration::ZERO
    } else {
      self.total_time / self.added_times as u32
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_add_single_time() {
    let mut times = BenchmarkTimes::new();
    times.add_time(Duration::from_millis(100));

    assert_eq!(times.average_times(), Duration::from_millis(100));
  }

  #[test]
  fn test_add_multiple_times_calculates_correct_average() {
    let mut times = BenchmarkTimes::new();
    times.add_time(Duration::from_millis(100));
    times.add_time(Duration::from_millis(200));
    times.add_time(Duration::from_millis(300));

    assert_eq!(times.average_times(), Duration::from_millis(200));
  }

  #[test]
  fn test_add_time_accumulates_total() {
    let mut times = BenchmarkTimes::new();
    times.add_time(Duration::from_secs(1));
    times.add_time(Duration::from_secs(2));
    times.add_time(Duration::from_secs(3));

    assert_eq!(times.average_times(), Duration::from_secs(2));
  }

  #[test]
  fn test_average_with_varying_durations() {
    let mut times = BenchmarkTimes::new();
    times.add_time(Duration::from_nanos(500));
    times.add_time(Duration::from_micros(1));
    times.add_time(Duration::from_millis(1));

    let average = times.average_times();
    assert!(average > Duration::ZERO);
  }
}
