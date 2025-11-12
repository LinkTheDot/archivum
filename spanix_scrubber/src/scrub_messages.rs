use crate::config::SpanixScrubberConfig;
use crate::response_models::avaiable_logs::{AvailableLogs, LogEntry};
use crate::response_models::user_messages::UserMessages;
use entities::twitch_user;
use futures::future::join_all;
use sea_orm::*;
use std::path::PathBuf;
use std::time::Instant;
use tokio::{fs, io::AsyncWriteExt};
use twitch_chat_tracker::errors::AppError;

impl SpanixScrubberConfig {
  pub async fn scrub_for_all_users_in_database_for_channel(self) -> ! {
    let data_output_dir = std::path::PathBuf::from(Self::DATA_OUTPUT_DIRECTORY);

    if !data_output_dir.exists() {
      fs::create_dir_all(data_output_dir).await.unwrap();
    } else if !data_output_dir.is_dir() {
      panic!("A file is taking up the path for the data output directory.");
    }

    let all_users = Self::get_users(self.database_connection).await.unwrap();
    let mut message_processing_futures = vec![];
    let total_user_count = all_users.len();

    for (iteration, user) in all_users.into_iter().enumerate() {
      let twitch_user::Model {
        login_name,
        twitch_id,
        ..
      } = user;

      tracing::info!("Processing messages for user {login_name} | {iteration}/{total_user_count}");

      let user_file_path = PathBuf::from(format!("{}/{twitch_id}", Self::DATA_OUTPUT_DIRECTORY));

      if Self::user_file_is_completed(&user_file_path, &login_name).await {
        tracing::warn!("Skipping messages for user `{login_name}`. File already exists.");

        continue;
      }

      let runtime = Instant::now();

      let Some(available_message_logs) = self.get_user_logs(&login_name).await else {
        continue;
      };

      let user_messages_result = self
        .get_available_entries(&login_name, available_message_logs)
        .await;
      let user_messages = match user_messages_result {
        Ok(user_messages) => user_messages,
        Err(error) => {
          tracing::error!("Failed to retrieve messages for user `{login_name}`. Reason: {error}");
          continue;
        }
      };

      let future = tokio::spawn(async move {
        Self::write_all_messages_to_user_file(login_name, twitch_id, user_messages).await
      });

      message_processing_futures.push(future);

      let run_time = runtime.elapsed();

      if run_time < Self::USER_ITERATION_TIME_LIMIT {
        let wait_time = Self::USER_ITERATION_TIME_LIMIT - run_time;

        tracing::info!(
          "Waiting on time limit to get next user. Time to wait: {}s",
          wait_time.as_secs()
        );

        tokio::time::sleep(wait_time).await;
      }
    }

    join_all(message_processing_futures).await;

    std::process::exit(0)
  }

  async fn get_users(
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<twitch_user::Model>, AppError> {
    twitch_user::Entity::find()
      .all(database_connection)
      .await
      .map_err(Into::into)
  }

  /// True is returned if the user has already been processed
  async fn user_file_is_completed(user_file_path: &PathBuf, login_name: &str) -> bool {
    if user_file_path.exists() {
      let contents = match fs::read_to_string(user_file_path).await {
        Ok(contents) => contents,
        Err(error) => {
          tracing::error!("Failed to check file for user `{login_name}`. Reason: {error}");

          return true;
        }
      };

      if contents.lines().last() == Some(Self::END_OF_FILE_INDICATOR) {
        return true;
      } else {
        tracing::warn!(
          "Found incomplete file for user `{login_name}`. Continuing with message retrieval."
        );
      }
    }

    false
  }

  /// Retrieves the available message logs for a user.
  ///
  /// None is returned if the list could not be retrieved or there were no logs.
  async fn get_user_logs(&self, login_name: &str) -> Option<AvailableLogs> {
    let available_message_logs_result =
      AvailableLogs::get_available_logs_for_user(login_name, &self.channel_login).await;
    let mut available_message_logs = match available_message_logs_result {
      Ok(Some(available_logs)) => available_logs,
      Ok(None) => {
        tracing::warn!("No available logs found for `{login_name}`.");

        return None;
      }
      Err(error) => {
        tracing::error!(
          "Failed to get available message logs for user {login_name}. Reason: {error}"
        );

        return None;
      }
    };

    available_message_logs.remove_after_date(Self::REMOVE_AFTER_YEAR, Self::REMOVE_AFTER_MONTH);

    if available_message_logs.logs.is_empty() {
      tracing::warn!("No available logs found for `{login_name}`.");

      return None;
    }

    Some(available_message_logs)
  }

  async fn get_available_entries(
    &self,
    login_name: &str,
    available_log_entries: AvailableLogs,
  ) -> Result<Vec<String>, AppError> {
    let mut all_user_messages = vec![];

    for LogEntry { year, month } in &available_log_entries.logs {
      tracing::info!("Getting messages on {year}-{month} for {login_name}");
      let raw_messages =
        UserMessages::get_messages(&self.channel_login, login_name, year, month).await?;
      let raw_messages: Vec<String> = raw_messages
        .messages
        .into_iter()
        .map(|user_message| user_message.raw)
        .collect();

      all_user_messages.extend(raw_messages);
    }

    Ok(all_user_messages)
  }

  /// Takes the list of raw IRC messages for a user and writes them to a file.
  async fn write_all_messages_to_user_file(
    user_login: String,
    user_twitch_id: i32,
    messages: Vec<String>,
  ) {
    let mut user_file_path = std::path::PathBuf::from(Self::DATA_OUTPUT_DIRECTORY);
    user_file_path.push(user_twitch_id.to_string());

    let open_user_file_result = fs::OpenOptions::new()
      .create(true)
      .truncate(true)
      .write(true)
      .open(user_file_path)
      .await;
    let mut user_file = match open_user_file_result {
      Ok(user_file) => user_file,
      Err(error) => {
        tracing::error!("Failed to open file for {user_login}. Reason: `{error}`");
        return;
      }
    };

    tracing::info!("Writing messages for {user_login} to file.");

    let mut serialized_messages = match serde_json::to_string(&messages) {
      Ok(messages) => messages,
      Err(error) => {
        tracing::error!(
          "Failed to parser user `{user_login}`'s messages to a json string. Reason: {error}"
        );
        return;
      }
    };

    serialized_messages.push_str(&format!("\n{}", Self::END_OF_FILE_INDICATOR));

    if let Err(error) = user_file.write_all(serialized_messages.as_bytes()).await {
      tracing::error!("Failed to write messages for user `{user_login}`'s file. Reason: {error}");
    }
  }
}
