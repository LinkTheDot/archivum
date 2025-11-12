use crate::errors::AppError;
use crate::websocket_connection::message_parser::WebsocketMessageParser;
use crate::websocket_connection::twitch_objects::stream_status::{
  StreamUpdateEventType, TwitchStreamUpdateMessage,
};
use entities::*;
use entity_extensions::prelude::*;
use sea_orm::*;

impl WebsocketMessageParser {
  /// Takes a [`JsonValue`](serde_json::Value) constructed from Twitch's Websocket connection for `stream.online` and `stream.offline` events.
  pub async fn parse_websocket_stream_status_update_message(
    message: JsonValue,
    database_connection: &DatabaseConnection,
  ) -> Result<(), AppError> {
    if message["metadata"]["message_type"] == "session_keepalive" {
      return Ok(());
    }

    let Ok(stream_update_message) =
      serde_json::from_value::<TwitchStreamUpdateMessage>(message.clone())
    else {
      return Err(AppError::FailedToParseValue {
        value_name: "stream status update",
        location: "parse websocket stream status update_message",
        value: format!("{:?}", message),
      });
    };

    match stream_update_message.get_subscription_event_type() {
      StreamUpdateEventType::Online => {
        let _stream_model = Self::stream_update_online(stream_update_message, database_connection)
          .await?
          .insert(database_connection)
          .await?;
      }

      StreamUpdateEventType::Offline => {
        Self::stream_update_offline(stream_update_message, database_connection)
          .await?
          .update(database_connection)
          .await?;
      }

      StreamUpdateEventType::Unknown => {
        return Err(AppError::UnknownEventTypeValueInStreamUpdateMessage {
          value: format!("{:?}", stream_update_message.get_subscription_event_type()),
        });
      }
    }

    Ok(())
  }

  async fn stream_update_online(
    stream_update_message: TwitchStreamUpdateMessage,
    database_connection: &DatabaseConnection,
  ) -> Result<stream::ActiveModel, AppError> {
    let Some(stream_id) = stream_update_message.get_stream_id() else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "stream twitch id",
        location: "parse websocket stream status update_message",
      });
    };
    let streamer = twitch_user::Model::get_or_set_by_twitch_id(
      stream_update_message.get_streamer_twitch_id(),
      database_connection,
    )
    .await?;
    let Some(start_time) = stream_update_message.get_started_at() else {
      return Err(AppError::MissingExpectedValue {
        expected_value_name: "stream start time",
        location: "parse websocket stream status update_message",
      });
    };

    Ok(stream::ActiveModel {
      twitch_stream_id: Set(stream_id),
      start_timestamp: Set(Some(start_time)),
      twitch_user_id: Set(streamer.id),
      ..Default::default()
    })
  }

  async fn stream_update_offline(
    stream_update_message: TwitchStreamUpdateMessage,
    database_connection: &DatabaseConnection,
  ) -> Result<stream::ActiveModel, AppError> {
    let streamer_twitch_id = stream_update_message.get_streamer_twitch_id();
    let streamer =
      twitch_user::Model::get_or_set_by_twitch_id(streamer_twitch_id, database_connection).await?;
    let Some(running_stream) =
      stream::Model::get_active_stream_for_user(&streamer, database_connection).await?
    else {
      return Err(
        AppError::FailedToFindActiveStreamForAUserWhereOneWasExpected {
          streamer_id: streamer.id,
        },
      );
    };
    let event_timestamp = stream_update_message.get_message_timestamp();

    let mut latest_stream_active_model = running_stream.into_active_model();

    latest_stream_active_model.end_timestamp = Set(Some(*event_timestamp));

    Ok(latest_stream_active_model)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::DateTime;

  #[tokio::test]
  async fn test_online_offline_websocket_message_parsing() {
    let online_message = online_websocket_message();
    let offline_message = offline_websocket_message();
    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_query_results([
        vec![twitch_user::Model {
          id: 1,
          twitch_id: 578762718,
          login_name: "fallenshadow".into(),
          display_name: "fallenshadow".into(),
        }],
        vec![twitch_user::Model {
          id: 1,
          twitch_id: 578762718,
          login_name: "fallenshadow".into(),
          display_name: "fallenshadow".into(),
        }],
      ])
      .append_query_results([vec![stream::Model {
        id: 1,
        twitch_stream_id: 1,
        start_timestamp: Some(
          DateTime::parse_from_rfc3339("2025-05-08T00:02:29.532137847Z")
            .unwrap()
            .to_utc(),
        ),
        end_timestamp: None,
        twitch_user_id: 1,
        title: None,
        twitch_vod_id: None,
      }]])
      .into_connection();

    let expected_online_active_model = stream::ActiveModel {
      id: NotSet,
      twitch_stream_id: Set(19136881),
      start_timestamp: Set(Some(
        DateTime::parse_from_rfc3339("2025-05-08T00:02:29.532137847Z")
          .unwrap()
          .to_utc(),
      )),
      end_timestamp: NotSet,
      twitch_user_id: Set(1),
      title: NotSet,
      twitch_vod_id: NotSet,
    };
    let expected_offline_active_model = stream::ActiveModel {
      id: Unchanged(1),
      twitch_stream_id: Unchanged(1),
      start_timestamp: Unchanged(Some(
        DateTime::parse_from_rfc3339("2025-05-08T00:02:29.532137847Z")
          .unwrap()
          .to_utc(),
      )),
      end_timestamp: Set(Some(
        DateTime::parse_from_rfc3339("2025-05-08T08:02:29.579998945Z")
          .unwrap()
          .to_utc(),
      )),
      twitch_user_id: Unchanged(1),
      title: Unchanged(None),
      twitch_vod_id: Unchanged(None),
    };

    let online_active_model =
      WebsocketMessageParser::stream_update_online(online_message, &mock_database)
        .await
        .unwrap();
    let offline_active_model =
      WebsocketMessageParser::stream_update_offline(offline_message, &mock_database)
        .await
        .unwrap();

    assert_eq!(online_active_model, expected_online_active_model);
    assert_eq!(offline_active_model, expected_offline_active_model);
    assert_eq!(online_active_model, expected_online_active_model);
  }

  fn online_websocket_message() -> TwitchStreamUpdateMessage {
    let json_data = r#"{
  "metadata": {
    "message_id": "1f6c8f83-0459-31a9-4fd9-e4fbe9997dc6",
    "message_timestamp": "2025-05-08T00:02:29.579998945Z",
    "message_type": "notification",
    "subscription_type": "stream.online",
    "subscription_version": "1"
  },
  "payload": {
    "event": {
      "broadcaster_user_id": "578762718",
      "broadcaster_user_login": "fallenshadow",
      "broadcaster_user_name": "fallenshadow",
      "id": "19136881",
      "started_at": "2025-05-08T00:02:29.532137847Z",
      "type": "live"
    },
    "subscription": {
      "condition": {
        "broadcaster_user_id": "578762718"
      },
      "cost": 0,
      "created_at": "2025-05-08T00:02:17.4288984Z",
      "id": "2d47fcf4-4232-7969-32d4-83833f1114dd",
      "status": "enabled",
      "transport": {
        "method": "websocket",
        "session_id": "73aea2ef_9d1c06eb"
      },
      "type": "stream.online",
      "version": "1"
    }
  }
}"#;

    serde_json::from_str::<TwitchStreamUpdateMessage>(json_data).unwrap()
  }

  fn offline_websocket_message() -> TwitchStreamUpdateMessage {
    let json_data = r#"{
  "metadata": {
    "message_id": "734fce04-be84-b905-89e5-54a23163c6ee",
    "message_timestamp": "2025-05-08T08:02:29.579998945Z",
    "message_type": "notification",
    "subscription_type": "stream.offline",
    "subscription_version": "1"
  },
  "payload": {
    "event": {
      "broadcaster_user_id": "578762718",
      "broadcaster_user_login": "fallenshadow",
      "broadcaster_user_name": "fallenshadow"
    },
    "subscription": {
      "condition": {
        "broadcaster_user_id": "578762718"
      },
      "cost": 0,
      "created_at": "2025-05-08T08:02:29.579998945Z",
      "id": "cef8051a-ce11-9f9e-f868-bb2785857834",
      "status": "enabled",
      "transport": {
        "method": "websocket",
        "session_id": "a891ed7c_32fef04a"
      },
      "type": "stream.offline",
      "version": "1"
    }
  }
}"#;

    serde_json::from_str::<TwitchStreamUpdateMessage>(json_data).unwrap()
  }
}
