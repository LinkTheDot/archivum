use super::mirrored_twitch_objects::message::TwitchIrcMessage;
use super::mirrored_twitch_objects::twitch_message_type::TwitchMessageType;
use crate::channel::third_party_emote_list_storage::EmoteListStorage;
use crate::errors::AppError;
use irc::proto::Message as IrcMessage;
use sea_orm::*;

mod bits_message_parsing;
mod gift_sub_message_parsing;
mod raid_message_parsing;
mod stream_message_parsing;
pub mod streamlabs_donation;
mod streamlabs_donation_message_parsing;
mod subscription_message_parsing;
mod timeout_message_parsing;

pub struct MessageParser<'a> {
  message: TwitchIrcMessage,
  third_party_emote_lists: &'a EmoteListStorage,
}

impl<'a> MessageParser<'a> {
  pub fn new(
    message: &IrcMessage,
    third_party_emote_lists: &'a EmoteListStorage,
  ) -> Result<Option<Self>, AppError> {
    let Some(message) = TwitchIrcMessage::new(message)? else {
      return Ok(None);
    };

    Ok(Some(Self {
      message,
      third_party_emote_lists,
    }))
  }

  pub async fn parse(self, database_connection: &DatabaseConnection) -> Result<(), AppError> {
    if self.message.message_type_has_user_message_attached() {
      self.parse_user_message(database_connection).await?;
    }

    match self.message.message_type() {
      TwitchMessageType::Bits => {
        self
          .parse_bits(database_connection)
          .await?
          .insert(database_connection)
          .await?;
      }
      TwitchMessageType::Subscription => {
        self
          .parse_subscription(database_connection)
          .await?
          .insert(database_connection)
          .await?;
      }
      TwitchMessageType::GiftSub => {
        self.parse_gift_subs(database_connection).await?;
      }
      TwitchMessageType::Timeout => {
        self
          .parse_timeout(database_connection)
          .await?
          .insert(database_connection)
          .await?;
      }
      TwitchMessageType::StreamlabsDonation => {
        self
          .parse_streamlabs_donation(database_connection)
          .await?
          .insert(database_connection)
          .await?;
      }
      TwitchMessageType::Raid => {
        self
          .parse_raid(database_connection)
          .await?
          .insert(database_connection)
          .await?;
      }
      _ => (),
    };

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::channel::third_party_emote_list_storage::EmoteListStorage;
  use crate::testing_helper_methods::timestamp_from_string;
  use entities::sea_orm_active_enums::EventType;
  use entities::{donation_event, stream_message, twitch_user};
  use irc::proto::message::Tag as IrcTag;
  use irc::proto::Message as IrcMessage;
  use irc::proto::{Command, Prefix};

  #[tokio::test]
  async fn bits_donation_parses_message_too() {
    let (bits_message, bit_donation_mock_database) = bits_donation_parses_message_too_template();
    let third_party_emote_storage = EmoteListStorage::test_list().unwrap();
    let message_parser = MessageParser::new(&bits_message, &third_party_emote_storage)
      .unwrap()
      .unwrap();

    message_parser
      .parse(&bit_donation_mock_database)
      .await
      .unwrap();
  }

  fn bits_donation_parses_message_too_template() -> (IrcMessage, DatabaseConnection) {
    let tags = vec![
      IrcTag("room-id".into(), Some("578762718".into())),
      IrcTag("user-id".into(), Some("128831052".into())),
      IrcTag("bits".into(), Some("100000".into())),
      IrcTag("tmi-sent-ts".into(), Some("1740956922774".into())),
      IrcTag("display-name".into(), Some("LinkTheDot".into())),
      IrcTag("login".into(), Some("linkthedot".into())),
      IrcTag("first-msg".into(), Some("0".into())),
      IrcTag("emote-only".into(), Some("0".into())),
      IrcTag("subscriber".into(), Some("1".into())),
    ];

    let message = IrcMessage {
      tags: Some(tags),
      prefix: Some(Prefix::Nickname(
        "linkthedot".into(),
        "linkthedot".into(),
        "linkthedot.tmi.twitch.tv".into(),
      )),
      command: Command::PRIVMSG("#fallenshadow".into(), "cheer100000".into()),
    };

    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_query_results([
        vec![twitch_user::Model {
          id: 1,
          twitch_id: 578762718,
          login_name: "fallenshadow".into(),
          display_name: "fallenshadow".into(),
        }],
        vec![],
        vec![twitch_user::Model {
          id: 3,
          twitch_id: 128831052,
          login_name: "linkthedot".into(),
          display_name: "LinkTheDot".into(),
        }],
      ])
      .append_query_results([vec![stream_message::Model {
        id: 1,
        is_first_message: 0_i8,
        timestamp: timestamp_from_string("1740956922774"),
        emote_only: 0_i8,
        contents: Some("cheer100000".to_string()),
        twitch_user_id: 3,
        channel_id: 1,
        stream_id: None,
        is_subscriber: 1_i8,
        origin_id: None,
      }]])
      .append_exec_results([MockExecResult {
        last_insert_id: 1,
        rows_affected: 1,
      }])
      .append_query_results([
        vec![twitch_user::Model {
          id: 1,
          twitch_id: 578762718,
          login_name: "fallenshadow".into(),
          display_name: "fallenshadow".into(),
        }],
        vec![],
        vec![twitch_user::Model {
          id: 3,
          twitch_id: 128831052,
          login_name: "linkthedot".into(),
          display_name: "LinkTheDot".into(),
        }],
      ])
      .append_query_results([vec![donation_event::Model {
        id: 1,
        event_type: EventType::Bits,
        amount: 100000.0_f32,
        timestamp: timestamp_from_string("1740956922774"),
        donator_twitch_user_id: Some(3),
        donation_receiver_twitch_user_id: 1,
        stream_id: None,
        subscription_tier: None,
        unknown_user_id: None,
        origin_id: None,
        source_id: None,
      }]])
      .append_exec_results([MockExecResult {
        last_insert_id: 1,
        rows_affected: 1,
      }])
      .into_connection();

    (message, mock_database)
  }
}
