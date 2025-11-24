use crate::{
  get_database_connection_as_arc, limited_database_connection::LimitedDatabaseConnection,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Debug, Clone)]
pub struct DatabaseConnectionManager {
  connection: Arc<DatabaseConnection>,
  semaphore: Arc<Semaphore>,
}

impl DatabaseConnectionManager {
  pub const CONNECTION_LIMIT: usize = 30;

  pub async fn new() -> Self {
    let connection = get_database_connection_as_arc().await;
    let semaphore = Arc::new(Semaphore::new(Self::CONNECTION_LIMIT));

    Self {
      connection,
      semaphore,
    }
  }

  /// Attempts to get a database connection, waiting until one is available if all connections are taken.
  pub async fn acquire(&self) -> LimitedDatabaseConnection {
    tracing::debug!("Getting database connection lock.");

    let permit = self.semaphore.clone().acquire_owned().await.unwrap();

    LimitedDatabaseConnection {
      connection: self.connection.clone(),
      _permit: permit,
    }
  }
}
