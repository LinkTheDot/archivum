use super::MessageParser;
use crate::errors::AppError;
use crate::irc_chat::message_parser::streamlabs_donation::StreamlabsDonation;
use crate::irc_chat::mirrored_twitch_objects::message::TwitchIrcMessage;
use crate::irc_chat::mirrored_twitch_objects::twitch_message_type::TwitchMessageType;
use entities::sea_orm_active_enums::EventType;
use entities::*;
use entity_extensions::prelude::*;
use irc::client::prelude::*;
use sea_orm::*;

impl MessageParser<'_> {
  pub async fn parse_streamlabs_donation(
    &self,
    database_connection: &DatabaseConnection,
  ) -> Result<donation_event::ActiveModel, AppError> {
    if self.message.message_type() != TwitchMessageType::StreamlabsDonation {
      return Err(AppError::IncorrectMessageType {
        expected_type: TwitchMessageType::StreamlabsDonation,
        got_type: self.message.message_type(),
      });
    }

    let Some(user_id) = self.message.user_id() else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "user id",
        location: "streamlabs donation parsing",
      });
    };

    if user_id != TwitchIrcMessage::STREAMELEMENTS_TWITCH_ID {
      return Err(AppError::IncorrectUserWhenParsingStreamlabsDonation {
        got_user: user_id.to_string(),
      });
    }

    let Command::PRIVMSG(_, message_contents) = &self.message.command() else {
      return Err(AppError::IncorrectCommandWhenParsingMessage {
        location: "streamlabs parser",
        command_string: format!("{:?}", self.message.command()),
      });
    };

    let Some(parsed_donation_contents) =
      StreamlabsDonation::parse_streamlabs_donation_value_from_message_content(message_contents)
    else {
      return Err(AppError::FailedToParseValue {
        value_name: "donation contents",
        location: "streamlabs donation parsing",
        value: message_contents.to_owned(),
      });
    };

    let donator = match twitch_user::Model::get_or_set_by_name(
      parsed_donation_contents.donator_name,
      database_connection,
    )
    .await
    {
      Ok(donator) => Some(donator),
      Err(error) => {
        tracing::error!(
          "Failed to get donator from a streamlabs donation. Reason: {:?}. Attempting guess based on known users.",
          error
        );

        twitch_user::Model::guess_name(parsed_donation_contents.donator_name, database_connection)
          .await?
      }
    };
    let unknown_user = if donator.is_none() {
      Some(
        unknown_user::Model::get_or_set_by_name(
          parsed_donation_contents.donator_name,
          database_connection,
        )
        .await?,
      )
    } else {
      None
    };

    let Some(streamer_twitch_id) = self.message.room_id() else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "room id",
        location: "bit donation parsing",
      });
    };
    let streamer_model =
      twitch_user::Model::get_or_set_by_twitch_id(streamer_twitch_id, database_connection).await?;
    let maybe_stream =
      stream::Model::get_active_stream_for_user(&streamer_model, database_connection).await?;

    let donation_event = donation_event::ActiveModel {
      event_type: Set(EventType::StreamlabsDonation),
      amount: Set(parsed_donation_contents.amount),
      timestamp: Set(*self.message.timestamp()),
      donator_twitch_user_id: Set(donator.map(|donator| donator.id)),
      unknown_user_id: Set(unknown_user.map(|user| user.id)),
      donation_receiver_twitch_user_id: Set(streamer_model.id),
      stream_id: Set(maybe_stream.map(|stream| stream.id)),
      source_id: Set(self.message.message_source_id().map(str::to_owned)),
      ..Default::default()
    };

    Ok(donation_event)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::channel::third_party_emote_list_storage::EmoteListStorage;
  use crate::testing_helper_methods::timestamp_from_string;
  use irc::proto::message::Tag as IrcTag;
  use irc::proto::Message as IrcMessage;
  use irc::proto::{Command, Prefix};

  #[tokio::test]
  async fn parse_streamlabs_donation_expected_value() {
    let (streamlabs_message, streamlabs_donation_mock_database) =
      get_streamlabs_donation_template();
    let third_party_emote_storage = EmoteListStorage::test_list().unwrap();
    let message_parser = MessageParser::new(&streamlabs_message, &third_party_emote_storage)
      .unwrap()
      .unwrap();

    let result = message_parser
      .parse_streamlabs_donation(&streamlabs_donation_mock_database)
      .await
      .unwrap();

    let expected_active_model = donation_event::ActiveModel {
      id: ActiveValue::NotSet,
      event_type: Set(EventType::StreamlabsDonation),
      amount: Set(143.0_f32),
      timestamp: Set(timestamp_from_string("1740956922774")),
      donator_twitch_user_id: Set(Some(3)),
      donation_receiver_twitch_user_id: Set(1),
      stream_id: Set(None),
      subscription_tier: ActiveValue::NotSet,
      unknown_user_id: Set(None),
      origin_id: ActiveValue::NotSet,
      source_id: Set(None),
    };

    assert_eq!(result, expected_active_model);
  }

  fn get_streamlabs_donation_template() -> (IrcMessage, DatabaseConnection) {
    let tags = vec![
      IrcTag("room-id".into(), Some("578762718".into())),
      IrcTag("tmi-sent-ts".into(), Some("1740956922774".into())),
      IrcTag("user-id".into(), Some("100135110".into())),
    ];

    let message = IrcMessage {
      tags: Some(tags),
      prefix: Some(Prefix::Nickname(
        "streamelements".into(),
        "streamelements".into(),
        "streamelements.tmi.twitch.tv".into(),
      )),
      command: Command::PRIVMSG(
        "#fallenshadow".into(),
        "5EVEN5 just tipped £143.00! thank you for the chocolate funds~ here's what they say: hopefully this covers cost of the imval april collection dress, or goes to paying for it".into(),
      ),
    };

    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_query_results([
        vec![twitch_user::Model {
          id: 3,
          twitch_id: 246216923,
          login_name: "5even5".into(),
          display_name: "5EVEN5".into(),
        }],
        vec![twitch_user::Model {
          id: 1,
          twitch_id: 578762718,
          login_name: "fallenshadow".into(),
          display_name: "fallenshadow".into(),
        }],
        vec![],
      ])
      .into_connection();

    (message, mock_database)
  }
}
