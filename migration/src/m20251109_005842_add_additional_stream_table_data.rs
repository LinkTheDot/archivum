use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    let add_vod_id_columns = Table::alter()
      .table(Stream::Table)
      .add_column(text(Stream::TwitchVodId).null())
      .to_owned();

    let add_stream_title_column = format!(
      "ALTER TABLE `{}` ADD COLUMN `{}` VARCHAR(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NULL",
      Stream::Table.to_string(),
      Stream::Title.to_string()
    );

    let create_muted_vod_segment_table = Table::create()
      .table(MutedVodSegment::Table)
      .if_not_exists()
      .col(integer(MutedVodSegment::StreamId).not_null())
      .col(integer(MutedVodSegment::Offset).not_null())
      .col(integer(MutedVodSegment::Duration).not_null())
      .primary_key(
        Index::create()
          .col(MutedVodSegment::StreamId)
          .col(MutedVodSegment::Offset),
      )
      .foreign_key(
        ForeignKey::create()
          .name("fk-muted_vod_segment-stream")
          .from(MutedVodSegment::Table, MutedVodSegment::StreamId)
          .to(Stream::Table, Stream::Id),
      )
      .to_owned();

    manager.alter_table(add_vod_id_columns).await?;

    manager
      .get_connection()
      .execute_unprepared(&add_stream_title_column)
      .await?;

    manager.create_table(create_muted_vod_segment_table).await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    let remove_stream_title_and_vod_id_columns = Table::alter()
      .table(Stream::Table)
      .drop_column(Stream::Title)
      .drop_column(Stream::TwitchVodId)
      .to_owned();

    let drop_muted_vod_segment_table = Table::drop().table(MutedVodSegment::Table).to_owned();

    manager
      .alter_table(remove_stream_title_and_vod_id_columns)
      .await?;

    manager.drop_table(drop_muted_vod_segment_table).await?;

    Ok(())
  }
}

#[derive(Iden)]
enum Stream {
  Table,
  Id,
  _TwitchUserId,
  _TwitchStreamId,
  _StartTimestamp,
  _EndTimestamp,
  Title,
  TwitchVodId,
}

#[derive(Iden)]
enum MutedVodSegment {
  Table,
  StreamId,
  Offset,
  Duration,
}
