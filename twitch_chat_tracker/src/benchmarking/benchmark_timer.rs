use crate::benchmarking::{
  benchmark_name::BenchmarkName, benchmark_times::BenchmarkTimes,
  running_benchmark_timer::RunningBenchmarkTimer,
};
use dashmap::DashMap;
use human_time::ToHumanTimeString;
use std::sync::OnceLock;
use std::time::Duration;

static BENCHMARKING_TIMER: OnceLock<BenchmarkTimer> = OnceLock::new();

#[derive(Default)]
pub struct BenchmarkTimer {
  benchmarks: DashMap<BenchmarkName, BenchmarkTimes>,
}

impl BenchmarkTimer {
  fn new() -> Self {
    BenchmarkTimer {
      benchmarks: DashMap::new(),
    }
  }

  fn get_or_set() -> &'static Self {
    BENCHMARKING_TIMER.get_or_init(Self::new)
  }

  pub fn start_timer(benchmark_name: BenchmarkName) -> RunningBenchmarkTimer {
    RunningBenchmarkTimer::new(benchmark_name)
  }

  pub fn finish(timer: RunningBenchmarkTimer) {
    let benchmark_name = *timer.benchmark_name();
    let finished_time = timer.stop();
    let benchmark_timer = Self::get_or_set();

    benchmark_timer
      .benchmarks
      .entry(benchmark_name)
      .or_default()
      .add_time(finished_time);
  }

  pub fn print_benchmark_data() {
    BenchmarkTimer::process_benchmark_data(|(benchmark_name, average_benchmark_runtime)| {
      tracing::info!(
        "Benchmark `{benchmark_name}`  |  Average runtime: `{}`",
        average_benchmark_runtime.to_human_time_string(),
      );
    })
  }

  /// Takes a closure on how to process the benchmark data.
  ///
  /// The closure takes each benchmark name and the average time it ran for.
  ///
  /// # Example
  /// ```no_run
  ///   BenchmarkTimer::print_data(|(benchmark_name, average_benchmark_runtime)| {
  ///     tracing::info!(
  ///       "Benchmark `{benchmark_name}`  |  Average runtime: `{}`",
  ///       average_benchmark_time.to_human_time_string(),
  ///     );
  ///   })
  /// ```
  fn process_benchmark_data<F>(benchmark_data: F)
  where
    F: Fn((BenchmarkName, Duration)),
  {
    let benchmark_timer = Self::get_or_set();

    for entry in benchmark_timer.benchmarks.iter() {
      let benchmark_name = *entry.key();
      let average_benchmark_time = entry.value().average_times();

      benchmark_data((benchmark_name, average_benchmark_time))
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::cell::RefCell;

  #[test]
  fn test_add_data_point_creates_timer() {
    let name = BenchmarkName::new("test1234");
    let timer = BenchmarkTimer::start_timer(name);

    assert_eq!(timer.benchmark_name(), &name);
  }

  #[test]
  fn test_add_data_point_tracks_benchmark() {
    let name = BenchmarkName::new("test1111");
    let timer = BenchmarkTimer::start_timer(name);
    BenchmarkTimer::finish(timer);

    let benchmark_timer = BenchmarkTimer::get_or_set();

    assert!(benchmark_timer.benchmarks.contains_key(&name));
  }

  #[tokio::test]
  async fn test_timer_returns_elapsed_duration() {
    let name = BenchmarkName::new("aaaaaaaaaa");
    let timer = BenchmarkTimer::start_timer(name);

    tokio::time::sleep(Duration::from_millis(10)).await;

    let elapsed = timer.stop();
    assert!(elapsed >= Duration::from_millis(10));
  }

  #[tokio::test]
  async fn test_print_data_calls_closure_with_benchmark_data() {
    let name = BenchmarkName::new("jlkasdjkladsajklsd");
    let timer = BenchmarkTimer::start_timer(name);

    tokio::time::sleep(Duration::from_millis(10)).await;
    let elapsed = timer.stop();

    let benchmark_timer = BenchmarkTimer::get_or_set();

    if let Some(mut times) = benchmark_timer.benchmarks.get_mut(&name) {
      times.add_time(elapsed);
    }

    let results = RefCell::new(Vec::new());

    BenchmarkTimer::process_benchmark_data(|(name, duration)| {
      results.borrow_mut().push((name, duration));
    });

    assert!(!results.borrow().is_empty());
  }
}
