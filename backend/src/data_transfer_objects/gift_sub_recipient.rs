use super::donation_event::DonationEventDto;
use crate::error::AppError;
use entities::*;
use sea_orm::*;

const INCLUDE_GIFT_SUB_RECIPIENT: bool = true;

#[derive(Debug, serde::Serialize)]
pub struct GiftSubRecipientDto {
  pub id: i32,
  pub recipient_months_subscribed: i32,
  pub recipient_twitch_user: Option<twitch_user::Model>,
  pub donation_event: DonationEventDto,
}

impl GiftSubRecipientDto {
  pub async fn from_gift_sub_recipient(
    gift_sub_recipient: gift_sub_recipient::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Self, AppError> {
    let twitch_user = Self::get_recipient(&gift_sub_recipient, database_connection).await?;
    let Some(donation_event) =
      donation_event::Entity::find_by_id(gift_sub_recipient.donation_event_id)
        .one(database_connection)
        .await?
    else {
      return Err(AppError::FailedToFindDonationEventByID {
        donation_event_id: gift_sub_recipient.donation_event_id,
      });
    };
    let donation_event_dto = DonationEventDto::from_donation_event(
      donation_event,
      INCLUDE_GIFT_SUB_RECIPIENT,
      database_connection,
    )
    .await?;

    Ok(Self {
      id: gift_sub_recipient.id,
      recipient_months_subscribed: gift_sub_recipient.recipient_months_subscribed,
      recipient_twitch_user: twitch_user,
      donation_event: donation_event_dto,
    })
  }

  pub async fn from_gift_sub_recipient_list(
    gift_sub_recipient_list: Vec<gift_sub_recipient::Model>,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<Self>, AppError> {
    let mut end_list = vec![];

    for gift_sub_recipient in gift_sub_recipient_list {
      let gift_sub_recipient_dto =
        Self::from_gift_sub_recipient(gift_sub_recipient, database_connection).await?;

      end_list.push(gift_sub_recipient_dto);
    }

    Ok(end_list)
  }

  async fn get_recipient(
    gift_sub_recipient: &gift_sub_recipient::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<twitch_user::Model>, AppError> {
    let Some(recipient_id) = gift_sub_recipient.twitch_user_id else {
      return Ok(None);
    };

    twitch_user::Entity::find_by_id(recipient_id)
      .one(database_connection)
      .await
      .map_err(Into::into)
  }

  pub async fn get_list_from_recipient_and_filter(
    gift_sub_recipient: twitch_user::Model,
    filter_for_channel: Option<twitch_user::Model>,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<Self>, AppError> {
    let mut gift_subs_received_query =
      gift_sub_recipient::Entity::find().find_also_related(donation_event::Entity);

    if let Some(filter_for_channel) = filter_for_channel {
      gift_subs_received_query = gift_subs_received_query
        .filter(donation_event::Column::DonationReceiverTwitchUserId.eq(filter_for_channel.id));
    }

    let gift_subs_received_with_donation_events = gift_subs_received_query
      .filter(gift_sub_recipient::Column::TwitchUserId.eq(gift_sub_recipient.id))
      .all(database_connection)
      .await?;

    let mut end_list = vec![];

    for (recipient, related_donation_event) in gift_subs_received_with_donation_events {
      let Some(related_donation_event) = related_donation_event else {
        tracing::warn!(
          "Donation receipient (ID: {}) is missing a related donation event.",
          recipient.id
        );
        continue;
      };
      let donation_event_dto = DonationEventDto::from_donation_event(
        related_donation_event,
        INCLUDE_GIFT_SUB_RECIPIENT,
        database_connection,
      )
      .await?;

      let recipient = GiftSubRecipientDto {
        id: recipient.id,
        recipient_months_subscribed: recipient.recipient_months_subscribed,
        recipient_twitch_user: Some(gift_sub_recipient.clone()),
        donation_event: donation_event_dto,
      };

      end_list.push(recipient);
    }

    Ok(end_list)
  }
}
