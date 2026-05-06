use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_index(
        Index::create()
          .name("idx_messages_channel_user_time")
          .table(StreamMessage::Table)
          .col(StreamMessage::ChannelId)
          .col(StreamMessage::TwitchUserId)
          .col(StreamMessage::Timestamp)
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_index(
        Index::drop()
          .name("idx_messages_channel_user_time")
          .table(StreamMessage::Table)
          .to_owned(),
      )
      .await
  }
}

#[derive(Iden)]
enum StreamMessage {
  Table,
  _Id,
  TwitchUserId,
  ChannelId,
  _StreamId,
  #[allow(clippy::enum_variant_names)] // Don't care.
  _IsFirstMessage,
  Timestamp,
  _EmoteOnly,
  _Contents,
  _ThirdPartyEmotesUsed,
  _IsSubscriber,
  _TwitchEmoteUsage,
  _OriginId,
}
