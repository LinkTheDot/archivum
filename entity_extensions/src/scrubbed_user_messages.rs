use crate::errors::EntityExtensionError;
use entities::scrubbed_user_messages;
use sea_orm::*;

pub trait ScrubbedUserMessagesExtensions {
  async fn update_completed_status(
    self,
    database_connection: &DatabaseConnection,
  ) -> Result<(), EntityExtensionError>;
}

impl ScrubbedUserMessagesExtensions for scrubbed_user_messages::Model {
  /// Sets the `completed_successfully` column to true for this instance of scrubbed_user_messages.
  async fn update_completed_status(
    self,
    database_connection: &DatabaseConnection,
  ) -> Result<(), EntityExtensionError> {
    let mut active_model = self.into_active_model();
    active_model.completed_successfully = Set(Some(true as i8));

    active_model.update(database_connection).await?;

    Ok(())
  }
}
