use crate::errors::AppError;
use std::{collections::VecDeque, time::Duration};
use tokio::time::Instant;
use tokio::{sync::mpsc, task::JoinHandle};

const MESSAGE_COUNT_INTERVAL: Duration = Duration::new(10, 0);
const MESSAGE_COUNT_SIZE: usize = 20;
const PRINT_INTERVAL: Duration = Duration::new(3600, 0);

#[derive(Debug)]
struct MessageCounter {
  /// Contains the shift instant used.
  counted_messages: VecDeque<(usize, Instant)>,
  last_shift: Instant,
  check_interval: Duration,
  log_timer: Instant,
}

pub async fn process_irc_message_results(
  mut message_parsing_handle_receiver: mpsc::UnboundedReceiver<JoinHandle<Result<bool, AppError>>>,
) {
  tracing::info!("Running message result process.");

  let mut message_counter = MessageCounter::new();

  while let Some(message_result) = message_parsing_handle_receiver.recv().await {
    message_counter.try_log();

    match message_result.await {
      Ok(Ok(true)) => {
        message_counter.increment();
        message_counter.try_log();
      }
      Ok(Err(error)) => tracing::error!("Failed to parse a message from the IRC client: {}", error),
      Err(error) => tracing::error!(
        "An error occurred when attempting to run a join handle: {}",
        error
      ),
      _ => (),
    }
  }

  tracing::error!("MPSC message parsing handle receiver has broken. Exiting.");

  // In the event where the connection fails, it's best to exit the program.
  std::process::exit(1)
}

impl MessageCounter {
  fn new() -> Self {
    let mut counted_messages = VecDeque::with_capacity(MESSAGE_COUNT_SIZE + 1);
    counted_messages.push_back((0, Instant::now()));

    Self {
      counted_messages,
      last_shift: Instant::now(),
      check_interval: MESSAGE_COUNT_INTERVAL,
      log_timer: Instant::now(),
    }
  }

  fn messages_per_minute(&mut self) -> usize {
    if self.counted_messages.len() < 2 {
      return 0;
    }

    let Some(oldest_time) = self.counted_messages.front().map(|m| m.1) else {
      return 0;
    };
    let Some(newest_time) = self.counted_messages.back().map(|m| m.1) else {
      return 0;
    };

    let seconds_elapsed = newest_time.duration_since(oldest_time).as_secs();

    if seconds_elapsed == 0 {
      return 0;
    }

    let total_count = self.total_messages();

    tracing::debug!(
      "Oldest: {}s  |  Newest: {}s",
      oldest_time.elapsed().as_secs(),
      newest_time.elapsed().as_secs()
    );
    tracing::debug!("total_count: {total_count}  |  time_elapsed: {seconds_elapsed}s");

    (total_count * 60) / seconds_elapsed as usize
  }

  fn increment(&mut self) {
    if self.last_shift.elapsed() >= self.check_interval {
      tracing::debug!("Shifting");

      let last_shift = std::mem::replace(&mut self.last_shift, Instant::now());

      self.counted_messages.push_back((0, last_shift));

      if self.counted_messages.len() > MESSAGE_COUNT_SIZE {
        self.counted_messages.pop_front();
      }
    }

    tracing::debug!("Incrementing");

    if let Some((count, _)) = self.counted_messages.back_mut() {
      *count += 1
    }
  }

  fn try_log(&mut self) {
    let messages_per_minute = self.messages_per_minute();

    if self.log_timer.elapsed() >= PRINT_INTERVAL && messages_per_minute > 0 {
      self.log_timer = Instant::now();

      tracing::info!(
        "Messages per minute: {}  | {} in count.",
        messages_per_minute,
        self.total_messages()
      );
    }
  }

  fn total_messages(&self) -> usize {
    self.counted_messages.iter().map(|(count, _)| count).sum()
  }
}
