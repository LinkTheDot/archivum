use clap::Parser;
use std::sync::OnceLock;

static ARGS: OnceLock<Args> = OnceLock::new();

#[derive(Parser)]
#[command(name = "TwitchChatTracker")]
pub struct Args {
  /// Runs the benchmark printing task on startup.
  #[arg(long)]
  print_benchmarks: bool,
}

impl Args {
  fn get_or_set() -> &'static Self {
    ARGS.get_or_init(Args::parse)
  }

  pub fn print_benchmarks() -> bool {
    Self::get_or_set().print_benchmarks
  }
}
