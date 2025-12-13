use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    todo!();
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    todo!();
  }
}

#[derive(Iden)]
enum Emote {
  Table,
  Id,
  _Name,
  _ExternalId,
  _ExternalService,
}
