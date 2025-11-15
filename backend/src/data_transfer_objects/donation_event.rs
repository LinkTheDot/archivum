use crate::data_transfer_objects::stream::StreamDto;
use crate::error::AppError;
use entities::*;
use entity::prelude::DateTimeUtc;
use sea_orm::*;
use sea_orm_active_enums::EventType;

#[derive(Debug, serde::Serialize)]
pub struct DonationEventDto {
  pub id: i32,
  pub event_type: EventType,
  pub amount: f32,
  pub timestamp: DateTimeUtc,
  pub donator: Option<twitch_user::Model>,
  pub donation_receiver: twitch_user::Model,
  pub stream: Option<StreamDto>,
  pub subscription_tier: Option<i32>,
  pub unknown_user: Option<unknown_user::Model>,
  pub origin_id: Option<String>,
  pub gift_sub_recipients: Option<Vec<twitch_user::Model>>,
}

impl DonationEventDto {
  pub async fn from_donation_event(
    donation_event: donation_event::Model,
    include_gift_sub_recipients: bool,
    database_connection: &DatabaseConnection,
  ) -> Result<Self, AppError> {
    let Some(donation_receiver) =
      twitch_user::Entity::find_by_id(donation_event.donation_receiver_twitch_user_id)
        .one(database_connection)
        .await?
    else {
      return Err(AppError::CouldNotFindUserByTwitchId {
        user_id: donation_event.donation_receiver_twitch_user_id.to_string(),
      });
    };
    let donator = Self::get_donator(&donation_event, database_connection).await?;
    let unknown_user = Self::get_unknown_user(&donation_event, database_connection).await?;
    let stream = Self::get_stream_dto(&donation_event, database_connection).await?;
    let gift_sub_recipients = if include_gift_sub_recipients {
      Self::get_gift_sub_recipients(&donation_event, database_connection).await?
    } else {
      None
    };

    Ok(Self {
      id: donation_event.id,
      event_type: donation_event.event_type,
      amount: donation_event.amount,
      timestamp: donation_event.timestamp,
      donator,
      donation_receiver,
      stream,
      subscription_tier: donation_event.subscription_tier,
      unknown_user,
      origin_id: donation_event.origin_id,
      gift_sub_recipients,
    })
  }

  pub async fn from_donation_event_list(
    donation_event_list: Vec<donation_event::Model>,
    include_gift_sub_recipients: bool,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<Self>, AppError> {
    let mut donation_event_dto_list = vec![];

    for donation_event in donation_event_list {
      let donation_event_dto = Self::from_donation_event(
        donation_event,
        include_gift_sub_recipients,
        database_connection,
      )
      .await?;

      donation_event_dto_list.push(donation_event_dto);
    }

    Ok(donation_event_dto_list)
  }

  async fn get_donator(
    donation_event: &donation_event::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<twitch_user::Model>, AppError> {
    let Some(donator_id) = donation_event.donator_twitch_user_id else {
      return Ok(None);
    };

    twitch_user::Entity::find_by_id(donator_id)
      .one(database_connection)
      .await
      .map_err(Into::into)
  }

  async fn get_unknown_user(
    donation_event: &donation_event::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<unknown_user::Model>, AppError> {
    let Some(unknown_user_id) = donation_event.unknown_user_id else {
      return Ok(None);
    };

    unknown_user::Entity::find_by_id(unknown_user_id)
      .one(database_connection)
      .await
      .map_err(Into::into)
  }

  async fn get_stream_dto(
    donation_event: &donation_event::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<StreamDto>, AppError> {
    let Some(stream_id) = donation_event.stream_id else {
      return Ok(None);
    };

    let Some(stream) = stream::Entity::find_by_id(stream_id)
      .one(database_connection)
      .await?
    else {
      return Ok(None);
    };

    StreamDto::from_stream(stream, database_connection)
      .await
      .map(Some)
  }

  async fn get_gift_sub_recipients(
    donation_event: &donation_event::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<Vec<twitch_user::Model>>, AppError> {
    if donation_event.event_type != EventType::GiftSubs {
      return Ok(None);
    }

    let gift_sub_recipients = gift_sub_recipient::Entity::find()
      .filter(gift_sub_recipient::Column::DonationEventId.eq(donation_event.id))
      .all(database_connection)
      .await?;
    let gift_sub_recipient_internal_ids: Vec<i32> = gift_sub_recipients
      .into_iter()
      .filter_map(|recipient| recipient.twitch_user_id)
      .collect();

    twitch_user::Entity::find()
      .filter(twitch_user::Column::Id.is_in(gift_sub_recipient_internal_ids))
      .all(database_connection)
      .await
      .map_err(Into::into)
      .map(Some)
  }
}
