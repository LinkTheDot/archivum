use super::MessageParser;
use crate::errors::AppError;
use crate::irc_chat::mirrored_twitch_objects::twitch_message_type::TwitchMessageType;
use entities::*;
use entity_extensions::donation_event::DonationEventExtensions;
use entity_extensions::prelude::*;
use sea_orm::*;
use sea_orm_active_enums::EventType;

impl MessageParser<'_> {
  pub async fn parse_gift_subs(
    &self,
    database_connection: &DatabaseConnection,
  ) -> Result<(), AppError> {
    let donation_event =
      if let Some(gift_sub_donation) = self.parse_gift_sub_message(database_connection).await? {
        gift_sub_donation.insert(database_connection).await?
      } else {
        self
          .donation_event_from_origin_id(database_connection)
          .await?
      };

    if self.message.gift_sub_has_recipient() {
      self
        .parse_gift_sub_recipient(donation_event, database_connection)
        .await?
        .insert(database_connection)
        .await?;
    }

    Ok(())
  }

  /// None is returned if the gift sub's origin id already exists in the database.
  async fn parse_gift_sub_message(
    &self,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<donation_event::ActiveModel>, AppError> {
    if self.message.message_type() != TwitchMessageType::GiftSub {
      return Err(AppError::IncorrectMessageType {
        expected_type: TwitchMessageType::GiftSub,
        got_type: self.message.message_type(),
      });
    }

    let Some(origin_id) = self.message.gift_sub_origin_id() else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "origin id",
        location: "gift sub parsing",
      });
    };

    let Some(gift_amount) = self.message.gift_sub_count() else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "gift sub count",
        location: "gift sub parsing",
      });
    };
    let Ok(gift_amount) = gift_amount.trim().parse::<f32>() else {
      return Err(AppError::FailedToParseValue {
        value_name: "gift sub count",
        location: "gift sub parsing",
        value: gift_amount.to_string(),
      });
    };

    if donation_event::Model::gift_sub_origin_id_already_exists(origin_id, database_connection)
      .await? && gift_amount == 1.0
    {
      return Ok(None);
    }

    let Some(streamer_twitch_id) = self.message.room_id() else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "room id",
        location: "gift sub parsing",
      });
    };
    let streamer_model =
      twitch_user::Model::get_or_set_by_twitch_id(streamer_twitch_id, database_connection).await?;
    let maybe_stream =
      stream::Model::get_active_stream_for_user(&streamer_model, database_connection).await?;
    let Some(donator_id) = self.message.user_id() else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "user id",
        location: "gift sub parsing",
      });
    };
    let donator =
      twitch_user::Model::get_or_set_by_twitch_id(donator_id, database_connection).await?;
    let Some(subscription_tier) = self.message.subscription_plan().cloned() else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "subscription plan",
        location: "gift sub parsing",
      });
    };

    let donation_event = donation_event::ActiveModel {
      event_type: Set(EventType::GiftSubs),
      amount: Set(gift_amount),
      timestamp: Set(*self.message.timestamp()),
      donator_twitch_user_id: Set(Some(donator.id)),
      donation_receiver_twitch_user_id: Set(streamer_model.id),
      stream_id: Set(maybe_stream.map(|stream| stream.id)),
      subscription_tier: Set(Some(subscription_tier.into())),
      origin_id: Set(Some(origin_id.into())),
      ..Default::default()
    };

    Ok(Some(donation_event))
  }

  async fn parse_gift_sub_recipient(
    &self,
    donation_event: donation_event::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<gift_sub_recipient::ActiveModel, AppError> {
    let Some(gift_sub_recipient_twitch_id) = self.message.gift_sub_recipient_twitch_id() else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "gift sub recipient twitch id",
        location: "parse gift sub recipient",
      });
    };
    let Some(recipient_months_subscribed) = self.message.gift_sub_recipient_months_subscribed()
    else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "gift sub recipient months subscribed",
        location: "parse gift sub recipient",
      });
    };
    let Ok(recipient_months_subscribed) = recipient_months_subscribed.parse::<i32>() else {
      return Err(AppError::FailedToParseValue {
        value_name: "recipient_months_subscribed",
        location: "parse gift sub recipient",
        value: recipient_months_subscribed.to_owned(),
      });
    };
    let gift_sub_recipient = twitch_user::Model::get_or_set_by_twitch_id(
      gift_sub_recipient_twitch_id,
      database_connection,
    )
    .await?;

    let gift_sub_recipient_active_model = gift_sub_recipient::ActiveModel {
      recipient_months_subscribed: Set(recipient_months_subscribed),
      twitch_user_id: Set(Some(gift_sub_recipient.id)),
      donation_event_id: Set(donation_event.id),
      ..Default::default()
    };

    Ok(gift_sub_recipient_active_model)
  }

  async fn donation_event_from_origin_id(
    &self,
    database_connection: &DatabaseConnection,
  ) -> Result<donation_event::Model, AppError> {
    let Some(origin_id) = self.message.gift_sub_origin_id() else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "origin id",
        location: "parse message",
      });
    };

    let Some(gift_sub_donation) =
      donation_event::Model::get_by_origin_id(origin_id, database_connection).await?
    else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "gift sub donation event",
        location: "parse message",
      });
    };

    Ok(gift_sub_donation)
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
  async fn parse_gift_subs_expected_value() {
    let (giftsub_message, giftsub_mock_database) = get_gift_subs_template(Some(5));
    let third_party_emote_storage = EmoteListStorage::test_list().unwrap();
    let message_parser = MessageParser::new(&giftsub_message, &third_party_emote_storage)
      .unwrap()
      .unwrap();

    let result = message_parser
      .parse_gift_sub_message(&giftsub_mock_database)
      .await
      .unwrap();

    let expected_active_model = donation_event::ActiveModel {
      id: ActiveValue::NotSet,
      event_type: Set(EventType::GiftSubs),
      amount: Set(5.0_f32),
      timestamp: Set(timestamp_from_string("1740956922774")),
      donator_twitch_user_id: Set(Some(3)),
      donation_receiver_twitch_user_id: Set(1),
      stream_id: Set(None),
      subscription_tier: Set(Some(1)),
      unknown_user_id: ActiveValue::NotSet,
      origin_id: Set(Some("1000".into())),
      source_id: NotSet,
    };

    assert_eq!(result, Some(expected_active_model));
  }
  #[tokio::test]
  async fn parse_gift_subs_no_sub_count_given() {
    let (giftsub_message, giftsub_mock_database) = get_gift_subs_template(None);
    let third_party_emote_storage = EmoteListStorage::test_list().unwrap();
    let message_parser = MessageParser::new(&giftsub_message, &third_party_emote_storage)
      .unwrap()
      .unwrap();

    let result = message_parser
      .parse_gift_sub_message(&giftsub_mock_database)
      .await
      .unwrap();

    let expected_active_model = donation_event::ActiveModel {
      id: ActiveValue::NotSet,
      event_type: Set(EventType::GiftSubs),
      amount: Set(1.0_f32),
      timestamp: Set(timestamp_from_string("1740956922774")),
      donator_twitch_user_id: Set(Some(3)),
      donation_receiver_twitch_user_id: Set(1),
      stream_id: Set(None),
      subscription_tier: Set(Some(1)),
      unknown_user_id: ActiveValue::NotSet,
      origin_id: Set(Some("1000".into())),
      source_id: NotSet,
    };

    assert_eq!(result, Some(expected_active_model));
  }
  fn get_gift_subs_template(sub_count: Option<i32>) -> (IrcMessage, DatabaseConnection) {
    let mut tags = vec![
      IrcTag("room-id".into(), Some("578762718".into())),
      IrcTag("user-id".into(), Some("128831052".into())),
      IrcTag("msg-param-sub-plan".into(), Some("1000".into())),
      IrcTag("tmi-sent-ts".into(), Some("1740956922774".into())),
      IrcTag("msg-id".into(), Some("submysterygift".into())),
      IrcTag("login".into(), Some("linkthedot".into())),
      IrcTag("display-name".into(), Some("LinkTheDot".into())),
      IrcTag("msg-param-origin-id".into(), Some("1000".into())),
    ];

    if let Some(sub_count) = sub_count {
      tags.push(IrcTag(
        "msg-param-mass-gift-count".into(),
        Some(sub_count.to_string()),
      ))
    }

    let message = IrcMessage {
      tags: Some(tags),
      prefix: Some(Prefix::ServerName("tmi.twitch.tv".into())),
      command: Command::Raw("USERNOTICE".into(), vec!["#fallenshadow".into()]),
    };

    let mut mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_query_results([
        vec![],
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
      .append_exec_results(vec![MockExecResult {
        last_insert_id: 1,
        rows_affected: 1,
      }])
      .append_query_results([vec![donation_event::Model {
        id: 1,
        event_type: EventType::GiftSubs,
        amount: 1.0_f32,
        timestamp: timestamp_from_string("1740956922774"),
        donator_twitch_user_id: Some(3),
        donation_receiver_twitch_user_id: 1,
        stream_id: None,
        subscription_tier: Some(1),
        unknown_user_id: None,
        origin_id: Some("1000".into()),
        source_id: None,
      }]]);

    for iteration in 0..sub_count.unwrap_or(0) {
      mock_database = mock_database.append_query_results([vec![twitch_user::Model {
        id: 10 + iteration,
        twitch_id: 100 + iteration,
        login_name: iteration.to_string(),
        display_name: iteration.to_string(),
      }]]);
    }

    (message, mock_database.into_connection())
  }
  #[tokio::test]
  async fn parse_gift_sub_recipients() {
    let (giftsub_message, database_connection) = get_gift_subs_template(Some(3));
    let gift_sub_recipients = get_gift_sub_recipient_messages(3);
    let third_party_emote_storage = EmoteListStorage::test_list().unwrap();
    let message_parser = MessageParser::new(&giftsub_message, &third_party_emote_storage)
      .unwrap()
      .unwrap();

    let donation_event_active_model = message_parser
      .parse_gift_sub_message(&database_connection)
      .await
      .unwrap()
      .unwrap();
    let donation_event_model = donation_event_active_model
      .insert(&database_connection)
      .await
      .unwrap();

    // First user that's been gifted.
    let message_parser = MessageParser::new(&gift_sub_recipients[0], &third_party_emote_storage)
      .unwrap()
      .unwrap();
    let gift_sub_recipient_active_model = message_parser
      .parse_gift_sub_recipient(donation_event_model.clone(), &database_connection)
      .await
      .unwrap();
    let expected_active_model = gift_sub_recipient::ActiveModel {
      id: NotSet,
      twitch_user_id: Set(Some(10)),
      recipient_months_subscribed: Set(1),
      donation_event_id: Set(1),
    };
    assert_eq!(gift_sub_recipient_active_model, expected_active_model);

    // Second user that's been gifted.
    let message_parser = MessageParser::new(&gift_sub_recipients[1], &third_party_emote_storage)
      .unwrap()
      .unwrap();
    let gift_sub_recipient_active_model = message_parser
      .parse_gift_sub_recipient(donation_event_model.clone(), &database_connection)
      .await
      .unwrap();
    let expected_active_model = gift_sub_recipient::ActiveModel {
      id: NotSet,
      twitch_user_id: Set(Some(11)),
      recipient_months_subscribed: Set(2),
      donation_event_id: Set(1),
    };
    assert_eq!(gift_sub_recipient_active_model, expected_active_model);

    // Third and final user that's been gifted.
    let message_parser = MessageParser::new(&gift_sub_recipients[2], &third_party_emote_storage)
      .unwrap()
      .unwrap();
    let gift_sub_recipient_active_model = message_parser
      .parse_gift_sub_recipient(donation_event_model.clone(), &database_connection)
      .await
      .unwrap();
    let expected_active_model = gift_sub_recipient::ActiveModel {
      id: NotSet,
      twitch_user_id: Set(Some(12)),
      recipient_months_subscribed: Set(3),
      donation_event_id: Set(1),
    };
    assert_eq!(gift_sub_recipient_active_model, expected_active_model);
  }
  fn get_gift_sub_recipient_messages(recipients: usize) -> Vec<IrcMessage> {
    let baseline_tags = vec![
      IrcTag("room-id".into(), Some("578762718".into())),
      IrcTag("msg-param-sub-plan".into(), Some("1000".into())),
      IrcTag("tmi-sent-ts".into(), Some("1740956922774".into())),
      IrcTag("msg-id".into(), Some("subgift".into())),
      IrcTag("msg-param-origin-id".into(), Some("1000".into())),
    ];

    (0..recipients)
      .map(|iteration| {
        let mut tags = baseline_tags.clone();

        tags.push(IrcTag(
          "user-id".into(),
          Some((100 + iteration).to_string()),
        ));
        tags.push(IrcTag("login".into(), Some(iteration.to_string())));
        tags.push(IrcTag("display-name".into(), Some(iteration.to_string())));
        tags.push(IrcTag(
          "msg-param-months".into(),
          Some((iteration + 1).to_string()),
        ));
        tags.push(IrcTag(
          "msg-param-recipient-id".into(),
          Some(iteration.to_string()),
        ));

        IrcMessage {
          tags: Some(tags),
          prefix: Some(Prefix::ServerName("tmi.twitch.tv".into())),
          command: Command::Raw("USERNOTICE".into(), vec!["#fallenshadow".into()]),
        }
      })
      .collect()
  }
  #[tokio::test]
  async fn mass_gift_sub_test() {
    let (giftsub_message, _) = get_gift_subs_template(Some(3));
    let third_party_emote_storage = EmoteListStorage::test_list().unwrap();
    let message_parser = MessageParser::new(&giftsub_message, &third_party_emote_storage)
      .unwrap()
      .unwrap();
    let expected_model = donation_event::Model {
      id: 1,
      event_type: EventType::GiftSubs,
      amount: 1.0_f32,
      timestamp: timestamp_from_string("1740956922774"),
      donator_twitch_user_id: Some(3),
      donation_receiver_twitch_user_id: 1,
      stream_id: None,
      subscription_tier: Some(1),
      unknown_user_id: None,
      origin_id: Some("1000".into()),
      source_id: None,
    };
    let expected_active_model = donation_event::ActiveModel {
      id: ActiveValue::NotSet,
      event_type: Set(EventType::GiftSubs),
      amount: Set(3.0_f32),
      timestamp: Set(timestamp_from_string("1740956922774")),
      donator_twitch_user_id: Set(Some(3)),
      donation_receiver_twitch_user_id: Set(1),
      stream_id: Set(None),
      subscription_tier: Set(Some(1)),
      unknown_user_id: ActiveValue::NotSet,
      origin_id: Set(Some("1000".into())),
      source_id: NotSet,
    };
    let (bulk_message, _) = get_gift_subs_template(None);
    let bulk_message_parser = MessageParser::new(&bulk_message, &third_party_emote_storage)
      .unwrap()
      .unwrap();

    let giftsub_mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_query_results([
        vec![],
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
      .append_query_results([
        vec![expected_model.clone()],
        vec![expected_model.clone()],
        vec![expected_model],
      ])
      .into_connection();

    let result = message_parser
      .parse_gift_sub_message(&giftsub_mock_database)
      .await
      .unwrap();

    assert_eq!(result, Some(expected_active_model));

    for _ in 0..3 {
      let result = bulk_message_parser
        .parse_gift_sub_message(&giftsub_mock_database)
        .await
        .unwrap();

      assert_eq!(result, None);
    }
  }
}
