mod extensions;
mod sea_orm_db_error_extensions;

pub use sea_orm_db_error_extensions::*;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
  #[error("")]
  ChannelQueriesPerMinuteExceeded,

  #[error("{0}")]
  UrlParseError(#[from] url::ParseError),

  #[error("{0}")]
  ReqwestError(#[from] reqwest::Error),

  #[error("{0}")]
  SerdeError(#[from] serde_json::Error),

  #[error("{0}")]
  EntityExtensionError(#[from] entity_extensions::errors::EntityExtensionError),

  #[error("{0}")]
  TungsteniteError(#[from] tungstenite::error::Error),

  #[error("Remaining Helix API requests is 0.")]
  ApiRatelimitReached,

  #[error("Failed to convert a reqwest header value into a string. Reason: `{:?}`", .0)]
  FailedToConvertHeaderValue(#[from] reqwest::header::ToStrError),

  #[error("Failed to query helix data for the user {:?}", .0)]
  UserDoesNotExist(String),

  #[error("Received an unknown response body structure when querying. Body location: {:?}", .0)]
  UnknownResponseBody(&'static str),

  #[error("Failed to configure the IRC client. Reason: `{:?}`", .0)]
  IrcError(#[from] irc::error::Error),

  #[error("Received nothing when polling for a message from the IRC client.")]
  NoIRCMessage,

  #[error("Attempted to repeat a GET request for a request that could not be cloned. Request: `{}`", .0)]
  RequestCouldNotBeCloned(String),

  #[error("{0}")]
  SeaOrmDbError(#[from] sea_orm::error::DbErr),

  #[error("Attempted to retrieve the global third party emote list, but couldn't find it.")]
  GlobalThirdPartyEmoteListIsMissing,

  #[error("Received a message from an unknown channel: {:?}", .0)]
  MessageFromUnknownChannel(String),

  #[error("Failed to convert unix timestamp {:?} to a proper timestamp.", .0)]
  CouldNotCreateTimestampWithUnixTimestamp(i64),

  #[error("Failed to retrieve the subscription plan from a sub/giftsub.")]
  NoSubscriptionPlan,

  #[error("Couldn't find a user's display name when parsing their message. Location: {:?}", .0)]
  FailedToGetUserName(&'static str),

  #[error("Encountered a Tokio IO error: `{:?}`", .0)]
  TokioIOError(#[from] tokio::io::Error),

  #[error("Got a message from a channel that wasn't being tracked. Channel Twitch ID: `{:?}`", .0)]
  GotMessageFromUntrackedChannel(i32),

  #[error("Received a donation for a channel that wasn't being tracked. Channel name: `{:?}`", .0)]
  DonationReceivedForUnknownChannel(String),

  #[error("Attempted to get the IRC client stream where there wasn't one.")]
  FailedToGetIrcClientStream,

  #[error("Attempted to get the IRC client where there wasn't one.")]
  FailedToGetIrcClient,

  /// When there's a missing value in the parser when one was expected.
  /// Say when you're parsing a subscription, but there was no subscription plan.
  ///
  /// Contains the value's name (something like "subscription plan"), and the location in the codebase the error occurred.
  #[error("Failed to retrieve {} at {}.", expected_value_name, location)]
  MissingExpectedValue {
    expected_value_name: &'static str,
    location: &'static str,
  },

  /// For when there's any issues parsing the Twitch ID of a user.
  ///
  /// Contains the place in the codebase the id couldn't be retrieved.
  #[error(
    "Failed to get the Twitch ID for user at: {:?}. Value: {:?}",
    location,
    value
  )]
  FailedToGetTwitchID {
    location: &'static str,
    value: String,
  },

  #[error("Attempted to query 7TV for a user's emote list, but got an error code back. {:?}", .0)]
  FailedToQuery7TVForEmoteList(String),

  #[error("Failed to deserialize a value. Reason: {:?}", .0)]
  DeserializeError(#[from] serde::de::value::Error),

  #[error("Failed to parse a string to time. Reason: {:?}", .0)]
  ChronoParseError(#[from] chrono::ParseError),

  #[error("Expected {} when parsing a message. Got {}", expected_type, got_type)]
  IncorrectMessageType {
    expected_type: crate::irc_chat::mirrored_twitch_objects::twitch_message_type::TwitchMessageType,
    got_type: crate::irc_chat::mirrored_twitch_objects::twitch_message_type::TwitchMessageType,
  },

  #[error("Failed to parse {} at {}. Got {:?}", value_name, location, value)]
  FailedToParseValue {
    value_name: &'static str,
    location: &'static str,
    value: String,
  },

  #[error(
    "Expected `streamelements` when parsing a donation, got `{}` instead",
    got_user
  )]
  IncorrectUserWhenParsingStreamlabsDonation { got_user: String },

  #[error(
    "Incorrect message format received at {}. Got command: {:?}",
    location,
    command_string
  )]
  IncorrectCommandWhenParsingMessage {
    location: &'static str,
    command_string: String,
  },

  #[error(
    "Failed to send message processing handle to the message processor: {}",
    error
  )]
  MpscConnectionClosed { error: String },

  #[error(
    "Failed to subscribe to an event for a channel. Subscription: {}, Response: {:?}",
    subscription_value,
    response
  )]
  FailedToGetEventSubSubscription {
    subscription_value: serde_json::Value,
    response: Option<String>,
  },

  #[error("Twitch has issued a close request.")]
  CloseRequested,

  #[error("The websocket connection has timedout.")]
  WebsocketTimeout,

  #[error(
    "Received an unknown value when parsing the event type for a websocket stream update message. Got: {:?}",
    value
  )]
  UnknownEventTypeValueInStreamUpdateMessage { value: String },

  #[error(
    "Failed to retrieve an active stream for user `{}` where one was expected.",
    streamer_id
  )]
  FailedToFindActiveStreamForAUserWhereOneWasExpected { streamer_id: i32 },

  #[error("Received a failed response from {}. Code: {}", location, code)]
  FailedResponse { location: &'static str, code: u16 },

  #[error("The IRC stream has been closed.")]
  IrcStreamClosed,
}
