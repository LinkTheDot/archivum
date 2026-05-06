use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(ChannelEmoteCache::Table)
          .if_not_exists()
          .primary_key(
            Index::create()
              .col(ChannelEmoteCache::TwitchUserId)
              .col(ChannelEmoteCache::EmoteId),
          )
          .col(integer(ChannelEmoteCache::TwitchUserId).not_null())
          .col(integer(ChannelEmoteCache::EmoteId).not_null())
          .foreign_key(
            ForeignKey::create()
              .name("fk-channel_emote_cache-twitch_user_id")
              .from(ChannelEmoteCache::Table, ChannelEmoteCache::TwitchUserId)
              .to(TwitchUser::Table, TwitchUser::Id)
              .on_delete(ForeignKeyAction::Cascade),
          )
          .foreign_key(
            ForeignKey::create()
              .name("fk-channel_emote_cache-emote_id")
              .from(ChannelEmoteCache::Table, ChannelEmoteCache::EmoteId)
              .to(Emote::Table, Emote::Id)
              .on_delete(ForeignKeyAction::Cascade),
          )
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table(ChannelEmoteCache::Table).to_owned())
      .await
  }
}

#[derive(Iden)]
enum ChannelEmoteCache {
  Table,
  TwitchUserId,
  EmoteId,
}

#[derive(Iden)]
enum TwitchUser {
  Table,
  Id,
  _TwitchId,
  _DisplayName,
  _LoginName,
}

#[derive(Iden)]
enum Emote {
  Table,
  Id,
  _Name,
  _ExternalId,
  _ExternalService,
}
