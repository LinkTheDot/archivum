use super::config::SpanixScrubberConfig;
use entities::twitch_user;
use entity_extensions::twitch_user::*;
use futures::future::join_all;
use irc::proto::Message as IrcMessage;
use irc::proto::message::Tag as IrcTag;
use sea_orm::*;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use twitch_chat_tracker::channel::third_party_emote_list_storage::EmoteListStorage;
use twitch_chat_tracker::irc_chat::message_parser::MessageParser;

const LOGIN_NAME_TAG: &str = "login";

impl SpanixScrubberConfig {
  pub async fn insert_user_messages_into_database(self, data_set: &str) -> ! {
    let third_party_emote_list = EmoteListStorage::new(
      std::slice::from_ref(&self.channel.login_name),
      self.database_connection,
    )
    .await
    .unwrap();
    let third_party_emote_list = Arc::new(third_party_emote_list);
    let mut data_path = PathBuf::from(Self::DATA_OUTPUT_DIRECTORY);
    data_path.push(data_set);
    let mut data_dir_entries = match fs::read_dir(&data_path).await {
      Ok(dir_entries) => dir_entries,
      Err(error) => {
        tracing::error!(
          "Failed to read entries for `{:?}`. Reason: {error}",
          data_path
        );

        std::process::exit(1)
      }
    };
    let mut message_processing_futures = vec![];

    while let Ok(Some(dir_entry)) = data_dir_entries.next_entry().await {
      let twitch_id = dir_entry.file_name();
      let twitch_id = twitch_id.to_str().unwrap().to_string();
      let file_path = dir_entry.path();

      let get_user_result = twitch_user::Model::get_by_identifier(
        ChannelIdentifier::TwitchID(&twitch_id),
        self.database_connection,
      )
      .await;
      let twitch_user::Model { login_name, .. } = match get_user_result {
        Ok(Some(user)) => user,
        Ok(None) => {
          tracing::error!(
            "Failed to get user of Twitch ID `{twitch_id}` because they did not exist."
          );
          continue;
        }
        Err(error) => {
          tracing::error!("Failed to get user of Twitch ID `{twitch_id}`. Reason: `{error}`");
          continue;
        }
      };

      let Some(serialized_irc_message_data) =
        Self::get_user_messages_from_file(file_path, &login_name).await
      else {
        continue;
      };

      let third_party_emote_list = third_party_emote_list.clone();
      let database_connection = self.database_connection;
      let process_messages_future = tokio::spawn(async {
        Self::process_messages(
          serialized_irc_message_data,
          login_name,
          third_party_emote_list,
          database_connection,
        )
        .await
      });

      message_processing_futures.push(process_messages_future)
    }

    tracing::info!("Waiting for all processes to finish.");

    let failed_messages_results = join_all(message_processing_futures).await;
    let failed_messages: Vec<IrcMessage> = failed_messages_results
      .into_iter()
      .flat_map(|failed_message_result| match failed_message_result {
        Ok(failed_messages) => failed_messages,
        Err(error) => {
          tracing::error!("Failed to retrieve failed messages from a future. Reason: `{error}`");

          vec![]
        }
      })
      .collect();

    self.handle_failed_messages(failed_messages, data_set).await;

    tracing::info!("Process finished.");

    std::process::exit(0)
  }

  /// Retrieves a user's messages from the given path.
  ///
  /// None is returned if the data could not be retrieved.
  async fn get_user_messages_from_file(
    file_path: PathBuf,
    login_name: &str,
  ) -> Option<Vec<String>> {
    let file_contents = match fs::read_to_string(&file_path).await {
      Ok(contents) => contents,
      Err(error) => {
        tracing::error!(
          "Failed to get file contents for user `{login_name}`. Path: {file_path:?}. Reason: {error}"
        );
        return None;
      }
    };

    if file_contents.lines().last() != Some(Self::END_OF_FILE_INDICATOR) {
      tracing::error!("Incomplete dataset for user `{login_name}`");
      return None;
    }

    let Some(data_line) = file_contents.lines().next() else {
      tracing::error!("No data in user `{login_name}`'s file.");
      return None;
    };
    let serialized_irc_message_data: Vec<String> = match serde_json::from_str(data_line) {
      Ok(serialized_data) => serialized_data,
      Err(error) => {
        tracing::error!("Failed to serialized data for user `{login_name}`. Reason: {error}");
        return None;
      }
    };

    Some(serialized_irc_message_data)
  }

  async fn handle_failed_messages(&self, failed_messages: Vec<IrcMessage>, data_set: &str) {
    if failed_messages.is_empty() {
      tracing::info!("No failed messages were found.");

      return;
    }

    let mut failed_message_file_path = PathBuf::from(Self::FAILED_MESSAGES_OUTPUT_DIRECTORY);

    if !failed_message_file_path.exists()
      && let Err(error) = fs::create_dir_all(&failed_message_file_path).await
    {
      tracing::error!(
        "Failed to create directory for failed user messages. Reason: `{error}`. Dumping failed messages."
      );

      tracing::info!("{failed_messages:?}");
    }

    failed_message_file_path.push(Self::FAILED_MESSAGES_FILE_NAME.replace("{data_set}", data_set));
    let mut end_string = String::new();

    for failed_message in failed_messages {
      let line = format!("{failed_message}");

      end_string.push_str(&line);
    }

    end_string.push_str(Self::END_OF_FILE_INDICATOR);

    let result = fs::write(failed_message_file_path, end_string.as_bytes()).await;

    if let Err(error) = result {
      tracing::error!("Failed to write failed messages to file. Reason: {error}");
    }
  }

  /// Processes all messages for a user, returning any failed ones.
  async fn process_messages(
    messages: Vec<String>,
    user_login: String,
    third_party_emote_list: Arc<EmoteListStorage>,
    database_connection: &'static DatabaseConnection,
  ) -> Vec<IrcMessage> {
    let message_count = messages.len();
    let percentages: Vec<usize> = (1..=10)
      .map(|divisor| (message_count as f32 * (divisor as f32 / 10.0)).ceil() as usize)
      .collect();
    let mut failed_messages = vec![];

    tracing::info!("Processing messages for {user_login}");

    for (iteration, message) in messages.into_iter().enumerate() {
      if percentages.contains(&iteration) {
        tracing::info!(
          "{}% ({iteration} | {message_count}) finished processing {user_login}'s messages",
          iteration * 100 / message_count
        );
      }
      let mut irc_message = IrcMessage::from(message.as_str());

      set_irc_login_tag_if_names_differ(&mut irc_message, &user_login);

      let message_parser = match MessageParser::new(&irc_message, &third_party_emote_list) {
        Ok(message_parser) => message_parser,
        Err(error) => {
          tracing::error!(
            "Failed to create message parser for message. Message: {message:?}. Reason: {error}"
          );

          failed_messages.push(irc_message);

          continue;
        }
      };

      if let Some(message_parser) = message_parser
        && let Err(error) = message_parser.parse(database_connection).await
      {
        tracing::error!("Failed to parse a message for user `{user_login}`. Reason: {error}");

        failed_messages.push(irc_message);
      }
    }

    tracing::info!("==Finished processing messages for {user_login}==");

    failed_messages
  }
}

/// Replaces the login tag value of the login name of the IRC message differs from the one given.
///
/// This is for when someone changes their username at some point.
fn set_irc_login_tag_if_names_differ(irc_message: &mut IrcMessage, user_login: &str) {
  let Some(tags) = &mut irc_message.tags else {
    return;
  };

  for IrcTag(tag_name, tag_value) in tags {
    if tag_name != LOGIN_NAME_TAG {
      continue;
    }

    if let Some(login) = tag_value
      && login != user_login
    {
      *login = user_login.to_string();
    }
  }
}
