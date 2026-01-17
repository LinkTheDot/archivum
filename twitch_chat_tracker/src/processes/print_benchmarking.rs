use benchmarking_tools::BenchmarkTimer;
use crate::clap::Args;
use std::time::Duration;

const PRINT_INTERVAL: Duration = Duration::new(60, 0);

/// Starts a loop that prints benchmarking data in fixed intervals.
///
/// Does not run if --print-benchmarks is false
pub async fn print_benchmarking() {
  if Args::print_benchmarks() {
    run().await;
  } else {
    tracing::warn!("Benchmark printing is off.");
  }
}

async fn run() -> ! {
  loop {
    tracing::info!("-= Logging benchmark data =-");
    BenchmarkTimer::print_benchmark_data();

    tokio::time::sleep(PRINT_INTERVAL).await;
  }
}
