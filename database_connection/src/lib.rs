use anyhow::anyhow;
use app_config::secret_string::Secret;
use app_config::AppConfig;
use migration::{Migrator, MigratorTrait, SchemaManager};
pub use sea_orm::DatabaseConnection;
use sea_orm::*;
use tokio::sync::OnceCell;

static DATABASE_CONNECTION: OnceCell<DatabaseConnection> = OnceCell::const_new();

pub async fn get_database_connection() -> &'static DatabaseConnection {
  DATABASE_CONNECTION
    .get_or_init(|| async { get_connection().await.unwrap() })
    .await
}

pub async fn get_owned_database_connection() -> DatabaseConnection {
  get_connection().await.unwrap()
}

async fn get_connection() -> anyhow::Result<sea_orm::DatabaseConnection> {
  let database_connection = Database::connect(database_connection_string(None))
    .await
    .unwrap();

  let _creation_exec_result = &match database_connection.get_database_backend() {
    DbBackend::MySql => database_connection
      .execute(Statement::from_string(
        database_connection.get_database_backend(),
        format!("CREATE DATABASE IF NOT EXISTS `{}`;", AppConfig::database()),
      ))
      .await
      .unwrap(),
    _ => panic!("Unsupported database backend."),
  };

  drop(database_connection);

  let database_connection =
    Database::connect(database_connection_string(Some(AppConfig::database())))
      .await
      .unwrap();

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
    "donation_event",
    "emote",
    "emote_usage",
    "gift_sub_recipient",
    "muted_vod_segments",
    "raid",
    "stream",
    "stream_message",
    "subscription_event",
    "twitch_user",
    "twitch_user_name_change",
    "twitch_user_unknown_user_association",
    "unknown_user",
    "user_timeout",
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
