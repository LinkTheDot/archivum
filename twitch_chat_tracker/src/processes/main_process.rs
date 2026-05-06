use crate::errors::AppError;
use crate::irc_chat::twitch_irc::TwitchIrc;
use std::time::Duration;
use tokio::{sync::mpsc, task::JoinHandle};

const RECONNECT_ATTEMPTS: usize = 10;

pub async fn run_main_process(
  message_result_processor_sender: mpsc::UnboundedSender<JoinHandle<Result<bool, AppError>>>,
) -> ! {
  tracing::info!("Starting main process.");

  let mut irc_client = TwitchIrc::new(message_result_processor_sender)
    .await
    .unwrap();

  tracing::info!("Running main process.");

  loop {
    let message_result = irc_client.next_message().await;

    match message_result {
      Err(AppError::IrcError(irc::error::Error::PingTimeout)) | Err(AppError::IrcStreamClosed) => {
        tracing::error!("=== PING TIMEOUT ERROR ===");

        if !reconnect_client(&mut irc_client, RECONNECT_ATTEMPTS).await {
          tracing::error!(
            "Failed to reconnect to Twitch's IRC servers after {} attempts. Exiting program.",
            RECONNECT_ATTEMPTS
          );

          std::process::exit(1);
        }
      }

      Err(AppError::MpscConnectionClosed { error }) => {
        tracing::error!(
          "Failed to send message processing handle to the message processor: {}. Exiting the program.",
          error
        );

        std::process::exit(1);
      }

      Err(AppError::IrcError(irc::error::Error::Io(error))) => {
        tracing::error!("Received an IO error: {:?}", error);

        if !reconnect_client(&mut irc_client, RECONNECT_ATTEMPTS).await {
          tracing::error!(
            "Failed to reconnect to Twitch's IRC servers after {} attempts. Exiting program.",
            RECONNECT_ATTEMPTS
          );

          std::process::exit(1);
        }
      }

      Err(error) => {
        tracing::error!("Failed to parse a message from the IRC client: `{}`", error);
      }

      _ => (),
    }
  }
}

/// Returns true if the client successfully reconnected.
///
/// False is returned if the client failed to reconnect after n attempts.
async fn reconnect_client(irc_client: &mut TwitchIrc, total_attempts: usize) -> bool {
  let mut attempts = 0;

  while let Err(error) = irc_client.reconnect().await {
    tracing::error!("Failed to reconnect the IRC client. Reason: `{:?}`", error);

    if attempts >= total_attempts {
      return false;
    }

    tokio::time::sleep(Duration::from_secs(10)).await;

    attempts += 1;
  }

  true
}
