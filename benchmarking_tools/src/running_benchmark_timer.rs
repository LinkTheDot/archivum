use crate::benchmark_name::BenchmarkName;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct RunningBenchmarkTimer {
  benchmark_name: BenchmarkName,
  timer: Instant,
}

impl RunningBenchmarkTimer {
  pub fn new(benchmark_name: BenchmarkName) -> Self {
    Self {
      benchmark_name,
      timer: Instant::now(),
    }
  }

  pub fn stop(self) -> Duration {
    self.timer.elapsed()
  }

  pub fn benchmark_name(&self) -> &BenchmarkName {
    &self.benchmark_name
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_timer_measures_elapsed_time() {
    let timer = RunningBenchmarkTimer::new(BenchmarkName::default());

    tokio::time::sleep(Duration::from_millis(10)).await;

    let elapsed = timer.stop();
    assert!(elapsed >= Duration::from_millis(10));
  }

  #[test]
  fn test_benchmark_name_accessor() {
    let name = BenchmarkName::default();
    let timer = RunningBenchmarkTimer::new(name);

    assert_eq!(timer.benchmark_name(), &name);
  }

  #[test]
  fn test_timer_starts_immediately() {
    let timer = RunningBenchmarkTimer::new(BenchmarkName::default());
    let elapsed = timer.stop();

    assert!(elapsed < Duration::from_millis(5));
  }
}
