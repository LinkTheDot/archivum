use sea_orm_migration::prelude::*;

const UNIQUE_EXTERNAL_SERVICE_EXTERNAL_ID_INDEX: &str = "idx_emote_external_service_external_id";
const OLD_EXTERNAL_ID_UNIQUE_KEY: &str = "external_id";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_index(
        Index::drop()
          .name(OLD_EXTERNAL_ID_UNIQUE_KEY)
          .table(Emote::Table)
          .to_owned(),
      )
      .await?;

    manager
      .create_index(
        Index::create()
          .name(UNIQUE_EXTERNAL_SERVICE_EXTERNAL_ID_INDEX)
          .table(Emote::Table)
          .col(Emote::ExternalId)
          .col(Emote::ExternalService)
          .unique()
          .to_owned(),
      )
      .await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_index(
        Index::drop()
          .name(UNIQUE_EXTERNAL_SERVICE_EXTERNAL_ID_INDEX)
          .table(Emote::Table)
          .to_owned(),
      )
      .await?;

    manager
      .create_index(
        Index::create()
          .name(OLD_EXTERNAL_ID_UNIQUE_KEY)
          .table(Emote::Table)
          .col(Emote::ExternalId)
          .unique()
          .to_owned(),
      )
      .await?;

    Ok(())
  }
}

#[derive(Iden)]
enum Emote {
  Table,
  _Id,
  _Name,
  ExternalId,
  ExternalService,
}
