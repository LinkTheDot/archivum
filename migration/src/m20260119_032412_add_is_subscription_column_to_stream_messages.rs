use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .alter_table(
        Table::alter()
          .table(StreamMessage::Table)
          .add_column(
            ColumnDef::new(StreamMessage::IsFromSubscriptionMessage)
              .boolean()
              .not_null()
              .default(false),
          )
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .alter_table(
        Table::alter()
          .table(StreamMessage::Table)
          .drop_column(StreamMessage::IsFromSubscriptionMessage)
          .to_owned(),
      )
      .await
  }
}

#[derive(Iden)]
enum StreamMessage {
  Table,
  _Id,
  _TwitchUserId,
  _ChannelId,
  _StreamId,
  #[allow(clippy::enum_variant_names)]
  _IsFirstMessage,
  _Timestamp,
  _EmoteOnly,
  _Contents,
  _ThirdPartyEmotesUsed,
  _IsSubscriber,
  _TwitchEmoteUsage,
  _OriginId,
  IsFromSubscriptionMessage,
}
