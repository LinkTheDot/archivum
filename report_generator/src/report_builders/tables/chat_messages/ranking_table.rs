use tabled::Tabled;

#[derive(Tabled, Debug, PartialEq, Eq)]
pub struct RankingEntry {
  pub place: String,
  pub name: String,
  pub messages_sent: usize,
  pub chat_percentage: String,
  pub avg_words_per_message: String,
  #[tabled(rename = "%_of_all_words")]
  pub percentage_of_all_words: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ChatRankings {
  pub all_messages: Vec<RankingEntry>,
  pub quality_filtered_messages: Vec<RankingEntry>,
}
