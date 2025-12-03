use std::sync::Arc;

use anyhow::anyhow;
use app_config::secret_string::Secret;
use app_config::AppConfig;
use migration::{Migrator, MigratorTrait, SchemaManager};
pub use sea_orm::DatabaseConnection;
use sea_orm::*;
use tokio::sync::OnceCell;

pub mod database_connection_manager;
pub mod limited_database_connection;

static DATABASE_CONNECTION: OnceCell<Arc<DatabaseConnection>> = OnceCell::const_new();

pub async fn get_database_connection() -> &'static DatabaseConnection {
  DATABASE_CONNECTION
    .get_or_init(|| async { Arc::new(get_connection().await.unwrap()) })
    .await
    .as_ref()
}

pub(crate) async fn get_database_connection_as_arc() -> Arc<DatabaseConnection> {
  DATABASE_CONNECTION
    .get_or_init(|| async { Arc::new(get_connection().await.unwrap()) })
    .await
    .clone()
}

pub async fn create_new_connection() -> DatabaseConnection {
  get_connection().await.unwrap()
}

async fn get_connection() -> anyhow::Result<sea_orm::DatabaseConnection> {
  let database_connection = Database::connect(database_connection_string(None)).await?;

  match database_connection.get_database_backend() {
    DbBackend::MySql => {
      database_connection
        .execute(Statement::from_string(
          database_connection.get_database_backend(),
          format!("CREATE DATABASE IF NOT EXISTS `{}`;", AppConfig::database()),
        ))
        .await?
    }
    _ => anyhow::bail!("Unsupported database backend."),
  };

  database_connection.close().await?;

  let mut opt = ConnectOptions::new(database_connection_string(Some(AppConfig::database())));
  opt.max_connections(50);

  let database_connection = Database::connect(opt).await?;

  run_migration(&database_connection).await?;

  Ok(database_connection)
}

fn database_connection_string(database_name: Option<&str>) -> String {
  let password = AppConfig::sql_user_password();
  let username = AppConfig::database_username();
  let address = AppConfig::database_address();
  let database = database_name.unwrap_or_default();

  format!(
    "mysql://{username}:{}@{address}/{database}",
    Secret::read_secret_string(password.read_value())
  )
}

async fn run_migration(database: &DatabaseConnection) -> anyhow::Result<()> {
  let schema_manager = SchemaManager::new(database);

  Migrator::up(database, None).await?;

  // Ensure that all expected tables exist before attempting to finish the migration.
  let check_tables = [
    entities::donation_event::Entity.table_name(),
    entities::emote::Entity.table_name(),
    entities::emote_usage::Entity.table_name(),
    entities::gift_sub_recipient::Entity.table_name(),
    entities::muted_vod_segment::Entity.table_name(),
    entities::raid::Entity.table_name(),
    entities::stream::Entity.table_name(),
    entities::stream_message::Entity.table_name(),
    entities::subscription_event::Entity.table_name(),
    entities::twitch_user::Entity.table_name(),
    entities::twitch_user_name_change::Entity.table_name(),
    entities::twitch_user_unknown_user_association::Entity.table_name(),
    entities::unknown_user::Entity.table_name(),
    entities::user_timeout::Entity.table_name(),
  ];

  for table_name in check_tables {
    if !schema_manager.has_table(table_name).await? {
      return Err(anyhow!(
        "Failed to migrate the database due to a missing table: `{:?}`",
        table_name
      ));
    }
  }

  Ok(())
}
