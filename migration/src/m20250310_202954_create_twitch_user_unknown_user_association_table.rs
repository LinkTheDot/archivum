use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(TwitchUserUnknownUserAssociation::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(TwitchUserUnknownUserAssociation::UnknownUserId)
              .integer()
              .not_null(),
          )
          .col(
            ColumnDef::new(TwitchUserUnknownUserAssociation::TwitchUserId)
              .integer()
              .not_null(),
          )
          .col(
            ColumnDef::new(UnknownUser::CreatedAt)
              .timestamp()
              .not_null()
              .default(Expr::current_timestamp()),
          )
          .primary_key(
            Index::create()
              .col(TwitchUserUnknownUserAssociation::TwitchUserId)
              .col(TwitchUserUnknownUserAssociation::UnknownUserId),
          )
          .foreign_key(
            ForeignKey::create()
              .name("fk-twitch_user_unknown_user_association-unknown_user_id")
              .from(
                TwitchUserUnknownUserAssociation::Table,
                TwitchUserUnknownUserAssociation::UnknownUserId,
              )
              .to(UnknownUser::Table, UnknownUser::Id)
              .on_delete(ForeignKeyAction::Cascade),
          )
          .foreign_key(
            ForeignKey::create()
              .name("fk-twitch_user_unknown_user_association-twitch_user_id")
              .from(
                TwitchUserUnknownUserAssociation::Table,
                TwitchUserUnknownUserAssociation::TwitchUserId,
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
      .drop_table(
        Table::drop()
          .table(TwitchUserUnknownUserAssociation::Table)
          .to_owned(),
      )
      .await
  }
}

#[derive(DeriveIden)]
enum TwitchUser {
  Table,
  Id,
  _TwitchId,
  _DisplayName,
  _LoginName,
}

#[derive(DeriveIden)]
enum UnknownUser {
  Table,
  Id,
  _Name,
  CreatedAt,
}

#[derive(DeriveIden)]
enum TwitchUserUnknownUserAssociation {
  Table,
  TwitchUserId,
  UnknownUserId,
  _CreatedAt,
}
