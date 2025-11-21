use std::collections::HashMap;
use entities::*;

#[derive(Debug, Default, Clone)]
pub struct UserMessageData<'a> {
  pub user_messages: HashMap<i32, UserMessages<'a>>,

  pub total_messages_sent: usize,
  pub total_emote_filtered_messages_sent: usize,

  pub total_words_sent: usize,
  pub emote_filtered_total_words_sent: usize,
}

#[derive(Debug, Default, Clone)]
pub struct UserMessages<'a> {
  pub all_messages: Vec<MessageWithWordCount<'a>>,
  pub emote_filtered_messages: Vec<MessageWithWordCount<'a>>,

  pub user_is_subscribed: bool,
  pub first_message_sent_this_stream: bool,
  pub total_words_sent: usize,
  pub total_words_sent_emote_filtered_messages: usize,
}

#[derive(Debug, Clone)]
pub struct MessageWithWordCount<'a> {
  pub stream_message: &'a stream_message::Model,
  pub word_count: usize,

  pub is_emote_dominant: bool,
}

impl<'a> UserMessages<'a> {
  /// Inserts the given message and updates all values based on the message.
  ///
  /// If the message is emote dominant it is also stored in its respective list.
  pub fn insert_message(&mut self, message: MessageWithWordCount<'a>) {
    self.total_words_sent += message.word_count;

    if message.stream_message.is_subscriber == 1 && !self.user_is_subscribed {
      self.user_is_subscribed = message.stream_message.is_subscriber == 1;
    }
    if message.stream_message.is_first_message == 1 {
      self.first_message_sent_this_stream = message.stream_message.is_first_message == 1
    }

    self.all_messages.push(message.clone());

    if !message.is_emote_dominant {
      self.total_words_sent_emote_filtered_messages += message.word_count;

      self.emote_filtered_messages.push(message)
    }
  }
}
