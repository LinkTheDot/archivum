use crate::conditions::query_conditions::AppQueryConditions;
use crate::errors::AppError;
use crate::report_builders::tables::chat_messages::messages_with_word_counts::UserMessageData;
use crate::EMOTE_DOMINANCE;
use database_connection::get_database_connection;
use entities::{emote_usage, stream_message, twitch_user};
use entity_extensions::emote_usage::EmoteUsageExtensions;
use messages_with_word_counts::{MessageWithWordCount, UserMessages};
use ranking_table::*;
use sea_orm::*;
use std::collections::HashMap;
use tabled::settings::Style;
use tabled::Table;
use tracing::instrument;

mod messages_with_word_counts;
mod ranking_table;

const QUALITY_WORD_COUNT_THRESHOLD: usize = 4;
const EMOTE_USAGE_BATCH_SIZE: usize = 30000;
const MESSAGE_QUALITY_INFO: &str = r#"
This table removes messages that are "low quality". The rules for a low quality messages are as follows:
 - More than {emote_message_threshold}% of the words were Twitch or third party emotes.
 - The message had less than {quality_message_word_threshold}
"#;
const WORD_PERCENTAGE_INFO: &str = "The `%_of_words` column shows how many of all words between all messages were from that particular user. Emotes are not counted as words.";
const USER_TAG_INFO: &str = r#"After a user's ranking will be indicators for both if they're subscribed and if they're a first time chatter.
* for first time chatter.
- for if the user isn't subscribed.
"#;

/// Returns the (leaderboard, quality_message_leaderboard) for a given stream.
///
/// Takes a condition to filter the messages by.
#[instrument(skip_all)]
pub async fn get_messages_sent_ranking(
  query_conditions: &AppQueryConditions,
  ranking_row_limit: Option<usize>,
) -> Result<(String, String), AppError> {
  let database_connection = get_database_connection().await;
  tracing::info!("Getting messages for message rankings.");
  let messages = stream_message::Entity::find()
    .filter(query_conditions.messages().clone())
    .all(database_connection)
    .await?;

  let rankings = calculate_rankings(&messages, database_connection, ranking_row_limit).await?;

  tracing::info!("Building chat ranking table strings.");

  let mut unfiltered_table = Table::new(rankings.all_messages);
  let mut filtered_table = Table::new(rankings.quality_filtered_messages);

  unfiltered_table.with(Style::markdown());
  filtered_table.with(Style::markdown());

  let quality_message_info = &MESSAGE_QUALITY_INFO
    .replace(
      "{emote_message_threshold}",
      &((EMOTE_DOMINANCE * 100.0).floor() as usize).to_string(),
    )
    .replace(
      "{quality_message_word_threshold}",
      &QUALITY_WORD_COUNT_THRESHOLD.to_string(),
    );

  let unfiltered_table =
    format!("{WORD_PERCENTAGE_INFO}\n\n{USER_TAG_INFO}\n\n{unfiltered_table}",);
  let filtered_table = format!(
    "{quality_message_info}\n{WORD_PERCENTAGE_INFO}\n\n{USER_TAG_INFO}\n\n{filtered_table}",
  );

  Ok((unfiltered_table, filtered_table))
}

async fn calculate_rankings(
  messages: &[stream_message::Model],
  database_connection: &DatabaseConnection,
  ranking_row_limit: Option<usize>,
) -> Result<ChatRankings, AppError> {
  tracing::info!("Calculating rankings for all messages.");

  let mut user_message_data: UserMessageData =
    group_user_message_data(messages, database_connection).await?;
  let mut chats_sent =
    replace_ids_with_users(&mut user_message_data.user_messages, database_connection).await?;
  chats_sent.sort_by(|(_, lhs), (_, rhs)| rhs.all_messages.len().cmp(&lhs.all_messages.len()));
  let mut quality_filtered_chats_sent = build_quality_filtered_chats(&chats_sent);

  if let Some(ranking_row_limit) = ranking_row_limit {
    tracing::info!("Truncating rankings to {ranking_row_limit} messages");

    chats_sent.truncate(ranking_row_limit);
    quality_filtered_chats_sent.truncate(ranking_row_limit);
  }

  let unfiltered_message_rankings =
    build_unfiltered_message_rankings(&user_message_data, chats_sent);
  let quality_filtered_message_rankings =
    build_quality_filtered_messages_rankings(&user_message_data, quality_filtered_chats_sent);

  Ok(ChatRankings {
    all_messages: unfiltered_message_rankings,
    quality_filtered_messages: quality_filtered_message_rankings,
  })
}

fn build_quality_filtered_chats<'a>(
  chats_sent: &[(twitch_user::Model, UserMessages<'a>)],
) -> Vec<(twitch_user::Model, UserMessages<'a>)> {
  let mut quality_filtered_chats = chats_sent.to_vec();

  quality_filtered_chats
    .retain(|(_user, chats_sent)| !chats_sent.quality_filtered_messages.is_empty());
  quality_filtered_chats.sort_by(|(_, lhs), (_, rhs)| {
    rhs
      .quality_filtered_messages
      .len()
      .cmp(&lhs.quality_filtered_messages.len())
  });

  quality_filtered_chats
}

fn build_unfiltered_message_rankings(
  user_message_data: &UserMessageData,
  chats_sent: Vec<(twitch_user::Model, UserMessages)>,
) -> Vec<RankingEntry> {
  let total_messages_sent = user_message_data.total_messages_sent;
  let total_word_count = user_message_data.total_words_sent;

  chats_sent
    .iter()
    .enumerate()
    .map(|(place, (user, user_messages))| {
      let mut place = (place + 1).to_string();
      let messages_sent = user_messages.all_messages.len();
      let chat_percentage = messages_sent as f32 / total_messages_sent as f32 * 100.0;
      let average_words_per_message = user_messages.total_words_sent as f32 / messages_sent as f32;
      let percentage_of_all_words = user_messages.total_words_sent as f32 / total_word_count as f32 * 100.0;

      // Sanity check just in case ids do not match
      let id_from_user = user.id;
      let id_from_message = user_messages.all_messages[0].stream_message.twitch_user_id;
      assert_eq!(
        id_from_user,
        id_from_message,
        "Mismatch in user ids detected when processing message rankings. {id_from_user} != {id_from_message}"
      );

      if user_messages.first_message_sent_this_stream {
        place.push('*')
      }
      if !user_messages.user_is_subscribed {
        place.push('-')
      }

      RankingEntry {
        place,
        name: user.login_name.clone(),
        messages_sent,
        chat_percentage: format!("{:.4}", chat_percentage),
        avg_words_per_message: format!("{:.2}", average_words_per_message),
        percentage_of_all_words: format!("{:.2}", percentage_of_all_words),
      }
    }).collect()
}

fn build_quality_filtered_messages_rankings(
  user_message_data: &UserMessageData,
  quality_filtered_chats_sent: Vec<(twitch_user::Model, UserMessages)>,
) -> Vec<RankingEntry> {
  let quality_filtered_messages_sent = user_message_data.total_quality_filtered_messages_sent;
  let total_quality_filtered_chats_word_count = user_message_data.quality_filtered_total_words_sent;

  quality_filtered_chats_sent
    .iter()
    .enumerate()
    .map(|(place, (user, user_messages))| {
      let mut place = (place + 1).to_string();
      let messages_sent = user_messages.quality_filtered_messages.len();
      let chat_percentage = messages_sent as f32 / quality_filtered_messages_sent as f32 * 100.0;
      let average_words_per_message =
        user_messages.total_words_sent_quality_filtered_messages as f32 / messages_sent as f32;
      let percentage_of_all_words = user_messages.total_words_sent_quality_filtered_messages as f32
        / total_quality_filtered_chats_word_count as f32
        * 100.0;

      if user_messages.first_message_sent_this_stream {
        place.push('*')
      }
      if !user_messages.user_is_subscribed {
        place.push('-')
      }

      RankingEntry {
        place,
        name: user.login_name.clone(),
        messages_sent,
        chat_percentage: format!("{:.4}", chat_percentage),
        avg_words_per_message: format!("{:.2}", average_words_per_message),
        percentage_of_all_words: format!("{:.2}", percentage_of_all_words),
      }
    })
    .collect()
}

async fn group_user_message_data<'a>(
  messages: &'a [stream_message::Model],
  database_connection: &DatabaseConnection,
) -> Result<UserMessageData<'a>, AppError> {
  tracing::info!("Grouping user message data.");

  tracing::info!("Getting emote usage for messages.");
  let emote_usage =
    emote_usage::Entity::load_many_batched(messages, database_connection, EMOTE_USAGE_BATCH_SIZE)
      .await?;

  tracing::info!("Grouping messages.");
  Ok(messages.iter().zip(emote_usage).fold(
    UserMessageData::default(),
    |mut grouped_user_messages, (message, emote_usage)| {
      let Some(message_contents) = &message.contents else {
        return grouped_user_messages;
      };

      let emotes_used: i32 = emote_usage
        .iter()
        .map(|emote_usage| emote_usage.usage_count)
        .sum();
      let word_count = message_contents
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .count() as i32;

      let real_words_count = (word_count - emotes_used).max(0) as usize;
      let is_emote_dominant = (emotes_used as f32 / word_count as f32) >= EMOTE_DOMINANCE;

      let message_with_word_count = MessageWithWordCount {
        stream_message: message,
        word_count: real_words_count,
        is_emote_dominant,
      };

      grouped_user_messages.insert_message(message_with_word_count, QUALITY_WORD_COUNT_THRESHOLD);

      grouped_user_messages
    },
  ))
}

async fn replace_ids_with_users<'a>(
  messages: &mut HashMap<i32, UserMessages<'a>>,
  database_connection: &DatabaseConnection,
) -> Result<Vec<(twitch_user::Model, UserMessages<'a>)>, AppError> {
  tracing::info!("Getting users for each set of messages.");

  let user_ids: Vec<i32> = messages.keys().copied().collect();

  let users = twitch_user::Entity::find()
    .filter(twitch_user::Column::Id.is_in(user_ids))
    .all(database_connection)
    .await?;

  Ok(
    users
      .into_iter()
      .filter_map(|user| {
        let Some(user_messages) = messages.remove(&user.id) else {
          tracing::error!("Failed to find user `{}` from message list.", user.id);

          return None;
        };

        Some((user, user_messages))
      })
      .collect(),
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::testing_helper_methods::*;
  use sea_orm::{DatabaseBackend, MockDatabase};

  #[tokio::test]
  async fn calculate_rankings_gives_expected_result() {
    let expected_user_query = vec![
      twitch_user::Model {
        id: 1,
        twitch_id: 1,
        login_name: "user1".into(),
        display_name: "user1".into(),
      },
      twitch_user::Model {
        id: 2,
        twitch_id: 2,
        login_name: "user2".into(),
        display_name: "user2".into(),
      },
      twitch_user::Model {
        id: 3,
        twitch_id: 3,
        login_name: "user3".into(),
        display_name: "user3".into(),
      },
    ];
    let (messages, emote_usage) = get_fake_stream_chat_logs();
    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_query_results([
        // emote_usage data
        emote_usage,
      ])
      .append_query_results([expected_user_query])
      .into_connection();

    let expected_chat_rankings = get_expected_chat_rankings();

    let chat_rankings = calculate_rankings(&messages, &mock_database, None)
      .await
      .unwrap();

    assert_eq!(chat_rankings, expected_chat_rankings);
  }

  /// Based on messages from `get_fake_stream_chat_logs`
  fn get_expected_chat_rankings() -> ChatRankings {
    let unfiltered_rankings = vec![
      RankingEntry {
        place: "1".into(),
        name: "user1".into(),
        messages_sent: 3,
        chat_percentage: format!("{:.4}", 3.0 / 8.0 * 100.0),
        avg_words_per_message: format!("{:.2}", 10.0 / 3.0),
        percentage_of_all_words: format!("{:.2}", 10.0 / 20.0 * 100.0),
      },
      RankingEntry {
        place: "2".into(),
        name: "user2".into(),
        messages_sent: 3,
        chat_percentage: format!("{:.4}", 3.0 / 8.0 * 100.0),
        avg_words_per_message: format!("{:.2}", 5.0 / 3.0),
        percentage_of_all_words: format!("{:.2}", 5.0 / 20.0 * 100.0),
      },
      RankingEntry {
        place: "3*-".into(),
        name: "user3".into(),
        messages_sent: 2,
        chat_percentage: format!("{:.4}", 2.0 / 8.0 * 100.0),
        avg_words_per_message: format!("{:.2}", 5.0 / 2.0),
        percentage_of_all_words: format!("{:.2}", 5.0 / 20.0 * 100.0),
      },
    ];
    let quality_filtered_rankings = vec![
      RankingEntry {
        place: "1".into(),
        name: "user1".into(),
        messages_sent: 2,
        chat_percentage: format!("{:.4}", 2.0 / 4.0 * 100.0),
        avg_words_per_message: format!("{:.2}", 10.0 / 2.0),
        percentage_of_all_words: format!("{:.2}", 10.0 / 19.0 * 100.0),
      },
      RankingEntry {
        place: "2".into(),
        name: "user2".into(),
        messages_sent: 1,
        chat_percentage: format!("{:.4}", 1.0 / 4.0 * 100.0),
        avg_words_per_message: format!("{:.2}", 4.0 / 1.0),
        percentage_of_all_words: format!("{:.2}", 4.0 / 19.0 * 100.0),
      },
      RankingEntry {
        place: "3*-".into(),
        name: "user3".into(),
        messages_sent: 1,
        chat_percentage: format!("{:.4}", 1.0 / 4.0 * 100.0),
        avg_words_per_message: format!("{:.2}", 5.0 / 1.0),
        percentage_of_all_words: format!("{:.2}", 5.0 / 19.0 * 100.0),
      },
    ];

    ChatRankings {
      all_messages: unfiltered_rankings,
      quality_filtered_messages: quality_filtered_rankings,
    }
  }

  fn get_fake_stream_chat_logs() -> (Vec<stream_message::Model>, Vec<emote_usage::Model>) {
    let mut first_time_message_not_subbed = generate_message(7, 3, "emote emote");
    first_time_message_not_subbed.is_first_message = 1;
    first_time_message_not_subbed.is_subscriber = 0;
    let mut second_message_not_subbed = generate_message(8, 3, "word in another message here");
    second_message_not_subbed.is_subscriber = 0;

    let messages = vec![
      generate_message(1, 1, "This is a message with words"),
      generate_message(2, 1, "emote emote This is another message"),
      generate_message(3, 1, "emote emote"),
      generate_message(4, 2, "message"),
      generate_message(5, 2, "emote emote word word word word"),
      generate_message(6, 2, "emote emote"),
      first_time_message_not_subbed,
      second_message_not_subbed,
    ];

    let emote_usage = vec![
      generate_emote_usage(1, 0, None),
      generate_emote_usage(2, 2, None),
      generate_emote_usage(3, 2, None),
      generate_emote_usage(4, 0, None),
      generate_emote_usage(5, 2, None),
      generate_emote_usage(6, 2, None),
      generate_emote_usage(7, 2, None),
      generate_emote_usage(8, 0, None),
    ];

    (messages, emote_usage)
  }
}
