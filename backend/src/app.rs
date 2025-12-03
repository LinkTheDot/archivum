use crate::error::*;
use database_connection::create_new_connection;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Clone)]
pub struct InterfaceConfig {
  database_connection: Arc<DatabaseConnection>,
}

impl InterfaceConfig {
  pub async fn new() -> Result<Self, AppError> {
    let database_connection = create_new_connection().await;

    Ok(Self {
      database_connection: Arc::new(database_connection),
    })
  }

  pub fn database_connection(&self) -> &DatabaseConnection {
    &self.database_connection
  }
}
