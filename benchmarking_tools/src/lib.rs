//! # Benchmarking Tools
//!
//! Benchmarking tools is a tool that contains a global state counter of average runtimes for labeled segments of code.
//!
//! Start with making a `BenchmarkTimer`:
//! ```
//! use benchmarking_tools::*;
//!
//! let benchmark_timer =                                                         
//!   BenchmarkTimer::start_timer(BenchmarkName("Segment benchmark name"));
//! ```
//!
//! This returns a timer that you end to enclose whatever segment of code you want to stop benchmarking like so:
//!
//! ```
//! use benchmarking_tools::*;
//!
//! let benchmark_timer =                                                         
//!   BenchmarkTimer::start_timer(BenchmarkName("Segment benchmark name"));
//!
//! // Code to benchmark here
//!
//! BenchmarkTimer::finish(benchmark_timer);
//! ```
//!
//! # Printing the results
//!
//! I suggest creating some task or thread to periotically print the results like so:
//!
//! ```
//! use benchmarking_tools::*;
//!
//! async fn run() -> ! {
//!   loop {
//!     tracing::info!("-= Logging benchmark data =-");
//!     BenchmarkTimer::print_benchmark_data();
//!
//!     tokio::time::sleep(PRINT_INTERVAL).await;
//!   }
//! }
//! ```
//!
//! This will print each actively running benchmark in order of creation like so:
//!
//! ```bash
//! Benchmark `benchmark name 1`  |  Average runetime: `31ms 568µs`
//! Benchmark `benchmark name 2`  |  Average runetime: `10µs`
//! Benchmark `benchmark name 3`  |  Average runetime: `3s 25ms`
//! ```

pub mod benchmark_name;
pub mod benchmark_timer;
pub mod benchmark_times;
pub mod running_benchmark_timer;

pub use benchmark_name::BenchmarkName;
pub use benchmark_timer::BenchmarkTimer;
