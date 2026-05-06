use crate::channel::third_party_emote_list_storage::EmoteListStorage;
use crate::errors::AppError;
use crate::irc_chat::message_parser::MessageParser;
use app_config::{secret_string::Secret, AppConfig};
use database_connection::database_connection_manager::DatabaseConnectionManager;
use irc::client::{prelude::*, ClientStream};
use irc::proto::{CapSubCommand, Message as IrcMessage};
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc, task::JoinHandle, time::timeout};
use tokio_stream::StreamExt;

const MESSAGE_WAIT_TIME: Duration = Duration::new(10, 0);

const TWITCH_IRC_SUBSCRIPTIONS: &str = "twitch.tv/tags twitch.tv/commands twitch.tv/membership";
const TWITCH_IRC_URL: &str = "irc.chat.twitch.tv";
const TWITCH_IRC_PORT: u16 = 6697;
const USE_TLS: bool = true;
/// In seconds.
const PING_TIMEOUT: u32 = 10;
/// In seconds.
const PING_TIME: u32 = 10;

pub struct TwitchIrc {
  irc_client: Client,
  irc_client_stream: Option<ClientStream>,
  third_party_emote_lists: Arc<EmoteListStorage>,
  message_result_processor_sender: mpsc::UnboundedSender<JoinHandle<Result<bool, AppError>>>,

  database_connection_manager: DatabaseConnectionManager,
}

impl TwitchIrc {
  pub async fn new(
    message_result_processor_sender: mpsc::UnboundedSender<JoinHandle<Result<bool, AppError>>>,
  ) -> Result<Self, AppError> {
    tracing::info!("Initializing Twitch IRC client.");
    let mut irc_client = Self::get_irc_client().await?;
    let irc_client_stream = irc_client.stream()?;
    let database_connection_manager = DatabaseConnectionManager::new().await;

    let third_party_emote_lists = {
      let database_connection = database_connection_manager.acquire().await;
      EmoteListStorage::new(AppConfig::channels(), &database_connection).await?
    };

    Ok(Self {
      irc_client,
      irc_client_stream: Some(irc_client_stream),
      third_party_emote_lists: Arc::new(third_party_emote_lists),
      message_result_processor_sender,
      database_connection_manager,
    })
  }

  pub async fn reconnect(&mut self) -> Result<(), AppError> {
    tracing::warn!("Reconnecting the IRC client.");

    self.irc_client = Self::get_irc_client().await?;

    let irc_client_stream = self.irc_client.stream()?;

    self.irc_client_stream = Some(irc_client_stream);

    tracing::info!("Successfully reconnected the IRC client");

    Ok(())
  }

  async fn get_irc_client() -> Result<Client, AppError> {
    let config = Self::get_config()?;
    let irc_client = Client::from_config(config).await?;
    irc_client.identify()?;

    irc_client.send(Command::CAP(
      None,
      CapSubCommand::REQ,
      Some(TWITCH_IRC_SUBSCRIPTIONS.to_string()),
      None,
    ))?;

    Ok(irc_client)
  }

  fn get_config() -> Result<Config, AppError> {
    let password = AppConfig::access_token().read_value();
    let password = Some("oauth:".to_string() + Secret::read_secret_string(password));

    Ok(Config {
      server: Some(TWITCH_IRC_URL.to_string()),
      nickname: Some(AppConfig::twitch_nickname().to_owned()),
      port: Some(TWITCH_IRC_PORT),
      password,
      use_tls: Some(USE_TLS),
      channels: Self::get_channels(),
      ping_timeout: Some(PING_TIMEOUT),
      ping_time: Some(PING_TIME),
      ..Default::default()
    })
  }

  fn get_channels() -> Vec<String> {
    AppConfig::channels()
      .iter()
      .map(|channel_name| {
        if !channel_name.starts_with("#") {
          format!("#{channel_name}")
        } else {
          channel_name.to_string()
        }
      })
      .collect()
  }

  fn get_mut_client_stream(&mut self) -> Result<&mut ClientStream, AppError> {
    self
      .irc_client_stream
      .as_mut()
      .ok_or(AppError::FailedToGetIrcClientStream)
  }

  pub async fn raw_next(&mut self) -> Result<Option<IrcMessage>, AppError> {
    let Ok(Some(message_result)) = timeout(
      Duration::from_secs(10),
      self.get_mut_client_stream()?.next(),
    )
    .await
    else {
      tracing::info!("Timed out with no message.");
      return Ok(None);
    };

    message_result.map(Some).map_err(Into::into)
  }

  /// Checks for the next message from the irc client stream.
  /// If no message is received within 10 seconds the function ends without doing anything.
  pub async fn next_message(&mut self) -> Result<(), AppError> {
    let future = self.get_mut_client_stream()?.next();
    let message_result = timeout(MESSAGE_WAIT_TIME, future).await;

    let message_result = match message_result {
      Ok(Some(message_result)) => message_result,
      Ok(None) => {
        return Err(AppError::IrcStreamClosed);
      }
      Err(_) => {
        return Ok(());
      }
    };

    let message = message_result?;

    tracing::debug!("Got a message: {message:?}");

    self.process_message(message).await
  }

  async fn process_message(&mut self, message: IrcMessage) -> Result<(), AppError> {
    if let Command::PING(url, _) = message.command {
      self.irc_client.send_pong(url)?;

      return Ok(());
    };

    let third_party_emote_lists = self.third_party_emote_lists.clone();
    tracing::debug!("Getting lock on database connection manager.");
    let database_connection_manager = self.database_connection_manager.clone();

    let process_message_future = Self::create_and_run_mesage_parser(
      message,
      database_connection_manager,
      third_party_emote_lists,
    );
    let process_message_handle = tokio::spawn(process_message_future);

    if let Err(error) = self
      .message_result_processor_sender
      .send(process_message_handle)
    {
      return Err(AppError::MpscConnectionClosed {
        error: error.to_string(),
      });
    }

    Ok(())
  }

  /// True is returned if the message was processed.
  async fn create_and_run_mesage_parser(
    message: IrcMessage,
    database_connection_manager: DatabaseConnectionManager,
    third_party_emote_lists: Arc<EmoteListStorage>,
  ) -> std::result::Result<bool, AppError> {
    tracing::debug!("Running message parsing.");

    match message.command {
      Command::JOIN(_, _, _) | Command::PART(_, _) => return Ok(false),
      Command::Response(_, _) => return Ok(false),
      Command::Raw(command, _) if &command == "USERSTATE" => return Ok(false),
      Command::Raw(command, _) if &command == "ROOMSTATE" => return Ok(false),
      Command::CAP(_, _, _, _) => return Ok(false),
      Command::PONG(ref _url, _) => return Ok(false),
      _ => (),
    }

    tracing::debug!("Parsing message tags.");

    let Some(message_parser) = MessageParser::new(&message, &third_party_emote_lists)? else {
      return Ok(false);
    };

    let database_connection = database_connection_manager.acquire().await;
    let result = message_parser.parse(&database_connection).await;

    if let Err(error) = &result {
      if !error.is_unique_constraint_violation() {
        tracing::error!(
          "Failed to process a message. Dumping contents to log.\n{:?}",
          message
        );
      } else {
        // Ignore the error if it's a unique constraint violation.
        return Ok(true);
      }
    }

    result.map(|_| true)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use database_connection::get_database_connection;
  use irc::proto::message::Tag as IrcTag;

  /// Used to manually test raw IRC messages from Twitch to
  /// check if the parser is working as intended.
  ///
  /// These tend to be tests that're either difficult to write unit tests for or
  /// for cases that weren't thought of before and therefor aren't being tested.
  #[tokio::test]
  #[ignore]
  async fn manual_message_testing() {
    crate::logging::setup_logging_config().unwrap();
    let message = IrcMessage {
      tags: Some(vec![
        IrcTag("display-name".to_string(), Some("guty_52".to_string())),
        IrcTag("badge-info".to_string(), Some("subscriber/23".to_string())),
        IrcTag("badges".to_string(), Some("subscriber/18,share-the-love/1".to_string())),
        IrcTag("color".to_string(), Some("#B22222".to_string())),
        IrcTag("display-name".to_string(), Some("max2fly".to_string())),
        IrcTag("emotes".to_string(), Some("".to_string())),
        IrcTag("flags".to_string(), Some("".to_string())),
        IrcTag("id".to_string(), Some("df133c4d-8be1-4bc8-91af-d7d1cb14adbc".to_string())),
        IrcTag("login".to_string(), Some("max2fly".to_string())),
        IrcTag("mod".to_string(), Some("0".to_string())),
        IrcTag("msg-id".to_string(), Some("resub".to_string())),
        IrcTag("msg-param-cumulative-months".to_string(), Some("23".to_string())),
        IrcTag("msg-param-months".to_string(), Some("0".to_string())),
        IrcTag("msg-param-multimonth-duration".to_string(), Some("1".to_string())),
        IrcTag("msg-param-multimonth-tenure".to_string(), Some("0".to_string())),
        IrcTag("msg-param-should-share-streak".to_string(), Some("1".to_string())),
        IrcTag("msg-param-streak-months".to_string(), Some("23".to_string())),
        IrcTag("msg-param-sub-plan-name".to_string(), Some("shondophrenics".to_string())),
        IrcTag("msg-param-sub-plan".to_string(), Some("Prime".to_string())),
        IrcTag("msg-param-was-gifted".to_string(), Some("false".to_string())),
        IrcTag("room-id".to_string(), Some("578762718".to_string())),
        IrcTag("subscriber".to_string(), Some("1".to_string())),
        IrcTag("system-msg".to_string(), Some("max2fly subscribed with Prime. They've subscribed for 23 months, currently on a 23 month streak!".to_string())),
        IrcTag("tmi-sent-ts".to_string(), Some("1768187076174".to_string())),
        IrcTag("user-id".to_string(), Some("71823941".to_string())),
        IrcTag("user-type".to_string(), Some("".to_string())),
        IrcTag("vip".to_string(), Some("0".to_string())),
      ]),
      prefix: Some(Prefix::ServerName("tmi.twitch.tv".into())),
      command: Command::Raw("USERNOTICE".into(), vec!["#fallenshadow".into(), "shondo you should add glorpnerd so I can bully link xdd".into()]),
      // Subscription no message
      // command: Command::Raw("USERNOTICE".into(), vec!["#fallenshadow".into()]),
    };
    let database_connection = get_database_connection().await;
    let third_party_emote_lists =
      EmoteListStorage::new(&["fallenshadow".to_string()], database_connection)
        .await
        .unwrap();

    MessageParser::new(&message, &third_party_emote_lists)
      .unwrap()
      .unwrap()
      .parse(database_connection)
      .await
      .unwrap();
  }
}
