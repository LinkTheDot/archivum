use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(ScrubbedUserMessages::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(ScrubbedUserMessages::CreatedAt)
              .timestamp()
              .not_null()
              .default(Expr::current_timestamp()),
          )
          .col(
            ColumnDef::new(ScrubbedUserMessages::TwitchUserId)
              .integer()
              .not_null(),
          )
          .col(
            ColumnDef::new(ScrubbedUserMessages::ChannelId)
              .integer()
              .not_null(),
          )
          .col(
            ColumnDef::new(ScrubbedUserMessages::CompletedSuccessfully)
              .boolean()
              .default(false)
              .not_null(),
          )
          .primary_key(
            Index::create()
              .col(ScrubbedUserMessages::TwitchUserId)
              .col(ScrubbedUserMessages::ChannelId),
          )
          .foreign_key(
            ForeignKey::create()
              .name("fk-scrubbed_user_messages-twitch_user_id")
              .from(
                ScrubbedUserMessages::Table,
                ScrubbedUserMessages::TwitchUserId,
              )
              .to(TwitchUser::Table, TwitchUser::Id)
              .on_delete(ForeignKeyAction::Cascade),
          )
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table(ScrubbedUserMessages::Table).to_owned())
      .await
  }
}

#[derive(Iden)]
enum ScrubbedUserMessages {
  Table,
  CreatedAt,
  TwitchUserId,
  ChannelId,
  CompletedSuccessfully,
}

#[derive(Iden)]
enum TwitchUser {
  Table,
  Id,
  _TwitchId,
  _DisplayName,
  _LoginName,
}
