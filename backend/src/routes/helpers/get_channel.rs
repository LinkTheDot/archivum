use entities::twitch_user;
use sea_orm::*;
use crate::error::AppError;

/// Guesses a user based on their login name.
pub async fn get_channel(
  channel_login: String,
  database_connection: &DatabaseConnection,
) -> Result<twitch_user::Model, AppError> {
  let get_channel_query =
    twitch_user::Entity::find().filter(twitch_user::Column::LoginName.contains(&channel_login));

  if let Some(channel) = get_channel_query.one(database_connection).await? {
    Ok(channel)
  } else {
    Err(AppError::CouldNotFindUserByLoginName {
      login: channel_login,
    })
  }
}
