use crate::data_transfer_objects::stream::StreamDto;
use crate::error::AppError;
use entities::*;
use entity::prelude::DateTimeUtc;
use sea_orm::*;

#[derive(Debug, serde::Serialize)]
pub struct SubscriptionEventDto {
  pub id: i32,
  pub months_subscribed: i32,
  pub timestamp: DateTimeUtc,
  pub channel: twitch_user::Model,
  pub stream: Option<StreamDto>,
  pub subscriber: Option<twitch_user::Model>,
  pub subscription_tier: Option<i32>,
}

impl SubscriptionEventDto {
  pub async fn from_subscription_event(
    subscription_event: subscription_event::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Self, AppError> {
    let Some(channel) = twitch_user::Entity::find_by_id(subscription_event.channel_id)
      .one(database_connection)
      .await?
    else {
      return Err(AppError::CouldNotFindUserByTwitchId {
        user_id: subscription_event.channel_id.to_string(),
      });
    };
    let subscriber = Self::get_subscriber(&subscription_event, database_connection).await?;
    let stream_dto = Self::get_stream_dto(&subscription_event, database_connection).await?;

    Ok(Self {
      id: subscription_event.id,
      months_subscribed: subscription_event.months_subscribed,
      timestamp: subscription_event.timestamp,
      channel,
      stream: stream_dto,
      subscriber,
      subscription_tier: subscription_event.subscription_tier,
    })
  }

  pub async fn from_subscription_event_list(
    subscription_events: Vec<subscription_event::Model>,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<Self>, AppError> {
    let mut subscription_event_dtos = vec![];

    for subscription_event in subscription_events {
      let subscription_event_dto =
        SubscriptionEventDto::from_subscription_event(subscription_event, database_connection)
          .await?;

      subscription_event_dtos.push(subscription_event_dto);
    }

    Ok(subscription_event_dtos)
  }

  async fn get_stream_dto(
    subscription_event: &subscription_event::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<StreamDto>, AppError> {
    let Some(stream_id) = subscription_event.stream_id else {
      return Ok(None);
    };

    let Some(stream) = stream::Entity::find_by_id(stream_id)
      .one(database_connection)
      .await?
    else {
      return Err(AppError::FailedToFindStreamByID {
        stream_id: stream_id as i64,
      });
    };

    StreamDto::from_stream(stream, database_connection)
      .await
      .map(Some)
  }

  async fn get_subscriber(
    subscription_event: &subscription_event::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<twitch_user::Model>, AppError> {
    let Some(subscriber_id) = subscription_event.subscriber_twitch_user_id else {
      return Ok(None);
    };

    twitch_user::Entity::find_by_id(subscriber_id)
      .one(database_connection)
      .await
      .map_err(Into::into)
  }
}
