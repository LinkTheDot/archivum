use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Allows the application to shut down gracefully between processing batches
/// rather than being killed mid-operation by SIGKILL.
///
/// Kubernetes sends SIGTERM before force-killing a pod. By listening for that
/// signal and checking it between batches, we ensure the current batch finishes
/// cleanly before the process exits.
#[derive(Clone)]
pub struct ShutdownSignal {
  requested: Arc<AtomicBool>,
}

impl ShutdownSignal {
  /// Creates a new `ShutdownSignal` and spawns a background task
  /// that listens for SIGTERM to trigger it.
  pub fn listen_for_sigterm() -> Self {
    let signal = Self {
      requested: Arc::new(AtomicBool::new(false)),
    };

    let flag = Arc::clone(&signal.requested);

    tokio::spawn(async move {
      let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();

      sigterm.recv().await;

      tracing::info!("SIGTERM received, shutting down after current batch");

      flag.store(true, Ordering::Relaxed);
    });

    signal
  }

  /// Returns true if a SIGTERM has been received since the signal listener was started.
  /// This should be checked between batch iterations to allow the current batch to
  /// finish before exiting.
  pub fn was_requested(&self) -> bool {
    self.requested.load(Ordering::Relaxed)
  }
}
