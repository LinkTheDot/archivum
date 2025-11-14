use crate::{
  channel::tracked_channels::TrackedChannels,
  errors::AppError,
  websocket_connection::{
    message_parser::WebsocketMessageParser, subscriptions::EventSubscription,
  },
};
use app_config::{secret_string::Secret, AppConfig};
use database_connection::get_database_connection;
use entities::twitch_user;
use entity_extensions::prelude::TwitchUserExtensions;
use futures_util::StreamExt;
use reqwest::{RequestBuilder, Response};
use sea_orm::DatabaseConnection;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::{net::TcpStream, time::timeout};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;

const WEBSOCKET_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
const TWITCH_API_URL: &str = "https://api.twitch.tv";

/// How long to wait between sending subscription events;
const SUBSCRIPTION_WAIT_TIME: Duration = Duration::new(0, 250000000);
const SUBSCRIPTION_PATH: &str = "helix/eventsub/subscriptions";
// - Use the below constants for the Twitch CLI connection. -
// const WEBSOCKET_URL: &str = "ws://127.0.0.1:8080/ws";
// const TWITCH_API_URL: &str = "http://127.0.0.1:8080/";
// const SUBSCRIPTION_PATH: &str = "eventsub/subscriptions";

/// The amount of messages to go through at startup to retrieve the session ID.
const GET_SESSION_ID_RETRY_ATTEMPTS: i32 = 5;
/// The amount of times to attmempt to subscribe to an event for a given channel.
pub const EVENT_SUBSCRIBE_RETRY_ATTEMPTS: i32 = 3;
/// As per the [documentation](https://dev.twitch.tv/docs/eventsub/handling-websocket-events/#subscription-limits)
const WEBSOCKET_SUBSCRIPTION_LIMIT: usize = 300;

/// Which events to subscribe to for each channel tracked.
const SUBSCRIPTIONS: &[EventSubscription] = &[
  // https://dev.twitch.tv/docs/eventsub/eventsub-subscription-types/#streamonline
  EventSubscription::new(None, "stream.online", 1),
  // https://dev.twitch.tv/docs/eventsub/eventsub-subscription-types/#streamoffline
  EventSubscription::new(None, "stream.offline", 1),
];
const SUBSCRIPTION_FAIL_RETRY_BASE_DURATION: Duration = Duration::new(30, 0);

/// In seconds.
///
/// Bound to 10-600 as per the documentation https://dev.twitch.tv/docs/eventsub/handling-websocket-events/
pub const KEEP_ALIVE_DURATION: u64 = 10;
/// How many extra seconds to wait for a keep alive notification.
pub const KEEP_ALIVE_GRACE_PERIOD: u64 = 3;

pub struct TwitchWebsocketConfig {
  keep_alive_timer: Instant,
  socket_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
  /// When a reconnect message is sent, the old stream will be stored here until a Welcome message is received in the new stream.
  session_id: String,
  tracked_channels: TrackedChannels,
}

impl TwitchWebsocketConfig {
  pub async fn new(
    tracked_channels: TrackedChannels,
    database_connection: &sea_orm::DatabaseConnection,
  ) -> Result<TwitchWebsocketConfig, AppError> {
    let mut url = Url::parse(WEBSOCKET_URL)?;
    let running_user =
      twitch_user::Model::get_or_set_by_name(AppConfig::twitch_nickname(), database_connection)
        .await?;
    url.query_pairs_mut().append_pair(
      "keepalive_timeout_seconds",
      &KEEP_ALIVE_DURATION.to_string(),
    );

    let (mut socket_stream, _) = connect_async(url.to_string()).await?;
    let Some(session_id) = Self::get_session_id(&mut socket_stream).await else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "session id",
        location: "new twitch websocket config",
      });
    };

    if Self::send_subscriptions_for_all_channels(
      tracked_channels.all_channels(),
      &running_user,
      &session_id,
    )
    .await?
    {
      tracing::error!("Failed to send all subscriptions successfully. Exiting the program.");

      std::process::exit(0);
    }

    Ok(Self {
      keep_alive_timer: Instant::now(),
      socket_stream,
      session_id,
      tracked_channels,
    })
  }

  /// Sends all subscriptions desired to Twitch for every channel in the [`app config`](app_config::AppConfig).
  ///
  /// If any subscription failed, true is returned.
  async fn send_subscriptions_for_all_channels(
    tracked_channels: Vec<&twitch_user::Model>,
    running_user: &twitch_user::Model,
    session_id: &str,
  ) -> Result<bool, AppError> {
    let mut url = Url::parse(TWITCH_API_URL)?;
    url.set_path(SUBSCRIPTION_PATH);

    let reqwest_client = reqwest::Client::new();
    let request = reqwest_client
      .post(url)
      .header(
        "Authorization",
        format!(
          "Bearer {}",
          Secret::read_secret_string(AppConfig::access_token().read_value())
        ),
      )
      .header(
        "Client-Id",
        Secret::read_secret_string(AppConfig::client_id().read_value()),
      )
      .header("Content-Type", "application/json");

    let subscription_bodies = EventSubscription::create_subscription_bodies_from_list(
      SUBSCRIPTIONS,
      tracked_channels,
      running_user,
      session_id,
    );

    if subscription_bodies.len() > WEBSOCKET_SUBSCRIPTION_LIMIT {
      panic!("Tracked user count is too high. Exceeded 300 subscription limit.");
    }

    let mut subscription_failed = false;

    for subscription in subscription_bodies {
      let request = request.try_clone().unwrap();
      let subscription_type = &subscription["type"].clone();

      let result = Self::send_subscription(subscription, request).await;

      match result {
        Ok(_response) => {}
        Err(response_error) => {
          tracing::error!(
            "Failed to process a POST request when subscribing to {}. Reason: {}",
            subscription_type,
            response_error
          );

          subscription_failed = true;
        }
      }

      tracing::info!("Sleeping for next subscription");
      tokio::time::sleep(SUBSCRIPTION_WAIT_TIME).await;
    }

    Ok(subscription_failed)
  }

  pub async fn send_subscription(
    value: Value,
    request_builder: RequestBuilder,
  ) -> Result<Response, AppError> {
    let mut attempts = EVENT_SUBSCRIBE_RETRY_ATTEMPTS;
    let request = request_builder.json(&value);

    while let Ok(response) = request.try_clone().unwrap().send().await {
      if attempts <= 0 {
        return Err(AppError::FailedToGetEventSubSubscription {
          subscription_value: value,
          response: Some(response.text().await?),
        });
      }

      if !response.status().is_success() {
        tracing::error!(
          "Failed to subscribe to an EventSub event. {attempts} remain. Response: {:?}",
          response
        );

        let sleep_multiplier = ((EVENT_SUBSCRIBE_RETRY_ATTEMPTS - attempts) + 1).max(0) as u32;
        let sleep_duration = SUBSCRIPTION_FAIL_RETRY_BASE_DURATION * sleep_multiplier;

        tokio::time::sleep(sleep_duration).await;

        attempts -= 1;

        continue;
      } else {
        return Ok(response);
      }
    }

    Err(AppError::FailedToGetEventSubSubscription {
      subscription_value: value,
      response: None,
    })
  }

  /// Extracts the session ID when connecting to Twitch's websocket servers.
  async fn get_session_id(
    socket_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
  ) -> Option<String> {
    let mut session_id: Option<String> = None;
    let mut attempts = GET_SESSION_ID_RETRY_ATTEMPTS;

    while let Some(message) = socket_stream.next().await {
      if let Ok(message) = message {
        if let Ok(message) = serde_json::from_str::<Value>(&message.to_string()) {
          if message["metadata"]["message_type"] == "session_welcome" {
            session_id = Some(message["payload"]["session"]["id"].to_string());

            break;
          }
        }
      }

      attempts -= 1;

      if attempts == 0 {
        break;
      }
    }

    if let Some(session_id) = &mut session_id {
      if session_id.starts_with('"') {
        session_id.remove(0);
      }

      if session_id.ends_with('"') {
        session_id.pop();
      }
    }

    session_id
  }

  /// Wait for the next message from the websocket connection, returning if none was received.
  ///
  /// If the message was for `stream.offline` or `stream.online` events, they are parsed, and the database is updated.
  ///
  /// Returns true if there was a message received.
  /// Otherwise, if [`keep_alive`](KEEP_ALIVE_DURATION) + [`grace`](KEEP_ALIVE_GRACE_PERIOD)
  /// seconds have passed, false is returned.
  ///
  /// Checks for:
  /// Keep alive: https://dev.twitch.tv/docs/eventsub/handling-websocket-events/#keepalive-message
  /// Reconnect: https://dev.twitch.tv/docs/eventsub/handling-websocket-events/#reconnect-message
  /// Revocation: https://dev.twitch.tv/docs/eventsub/handling-websocket-events/#revocation-message
  /// Close: https://dev.twitch.tv/docs/eventsub/handling-websocket-events/#close-message
  pub async fn check_for_stream_message(&mut self) -> Result<bool, AppError> {
    let check_message_future = self.socket_stream.next();
    let message_result = timeout(
      Duration::from_secs(KEEP_ALIVE_DURATION + KEEP_ALIVE_GRACE_PERIOD),
      check_message_future,
    )
    .await;

    let Ok(Some(message_result)) = message_result else {
      tracing::debug!("Did not recieve a message.");

      return Ok(false);
    };

    self.update_keep_alive()?;

    let message = message_result?;

    if message.is_close() {
      tracing::error!("Twitch has issued a close request. Message: {:?}", message);

      return Err(AppError::CloseRequested);
    }

    let message = message.to_text()?;

    if message.is_empty() {
      return Ok(true);
    }

    let Ok(message) = serde_json::from_str::<Value>(message) else {
      return Err(AppError::FailedToParseValue {
        value_name: "message",
        location: "websocket config next",
        value: message.to_string(),
      });
    };

    if message["metadata"]["message_type"] == "session_reconnect" {
      let reconnect_url = Self::reconnect_url_from_message_payload(&message);

      self.reconnect_with_url(reconnect_url).await?;

      return Ok(true);
    }

    WebsocketMessageParser::parse_websocket_stream_status_update_message(
      message,
      get_database_connection().await,
    )
    .await?;

    Ok(true)
  }

  pub async fn restart(
    &mut self,
    database_connection: &DatabaseConnection,
  ) -> Result<(), AppError> {
    tracing::warn!("The websocket client is being restarted.");

    let new_connection = Self::new(self.tracked_channels.clone(), database_connection).await?;

    let _ = std::mem::replace(self, new_connection);

    Ok(())
  }

  /// Reconnects with the given reconnect URL provided by Twitch in their reconnect message.
  ///
  /// Reconnects as per their documentation: https://dev.twitch.tv/docs/eventsub/handling-websocket-events/#reconnect-message
  ///
  /// Exits the program if there was not welcome message provided.
  async fn reconnect_with_url(&mut self, reconnect_url: String) -> Result<(), AppError> {
    let (mut new_socket_stream, _) = connect_async(&reconnect_url).await?;
    let Some(session_id) = Self::get_session_id(&mut new_socket_stream).await else {
      let error = AppError::MissingExpectedValue {
        expected_value_name: "session id",
        location: "websocket config reconnect",
      };

      tracing::error!("{}, Closing the program.", error);

      std::process::exit(1);
    };

    self.session_id = session_id;

    let _ = std::mem::replace(&mut self.socket_stream, new_socket_stream);

    Ok(())
  }

  fn reconnect_url_from_message_payload(message: &Value) -> String {
    let mut reconnect_url = message["payload"]["session"]["reconnect_url"].to_string();

    if reconnect_url.starts_with('"') {
      reconnect_url.remove(0);
    }
    if reconnect_url.ends_with('"') {
      reconnect_url.pop();
    }

    reconnect_url
  }

  /// Returns AppError::WebsocketTimeout if the duration in the keep alive timer is larger than [`KEEP_ALIVE_DURATION`](TwitchWebsocketConfig::KEEP_ALIVE_DURATION)
  ///
  /// This should be called after every message. In the case where no events are received, Twitch will send a keep alive message: https://dev.twitch.tv/docs/eventsub/handling-websocket-events/#keepalive-message
  fn update_keep_alive(&mut self) -> Result<(), AppError> {
    if self.keep_alive_timer.elapsed()
      > Duration::from_secs(KEEP_ALIVE_DURATION + KEEP_ALIVE_GRACE_PERIOD)
    {
      return Err(AppError::WebsocketTimeout);
    } else {
      self.keep_alive_timer = Instant::now();
    }

    Ok(())
  }
}
