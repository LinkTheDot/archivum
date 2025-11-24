use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::sync::{OwnedSemaphorePermit};

pub struct LimitedDatabaseConnection {
  pub(crate) connection: Arc<DatabaseConnection>,
  pub(crate) _permit: OwnedSemaphorePermit,
}

impl std::ops::Deref for LimitedDatabaseConnection {
  type Target = DatabaseConnection;

  fn deref(&self) -> &Self::Target {
    &self.connection
  }
}
