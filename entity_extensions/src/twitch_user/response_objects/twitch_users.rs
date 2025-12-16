use crate::errors::EntityExtensionError;
use entities::twitch_user;
use sea_orm::Set;

#[derive(Debug, serde::Deserialize)]
pub struct TwitchUserListResponse {
  pub data: Vec<TwitchUserResponse>,
}

#[derive(Debug, serde::Deserialize)]
pub struct TwitchUserResponse {
  pub id: String,
  pub login: String,
  pub display_name: String,
}

impl TryFrom<TwitchUserResponse> for twitch_user::ActiveModel {
  type Error = EntityExtensionError;

  fn try_from(user_response: TwitchUserResponse) -> Result<Self, Self::Error> {
    let Ok(twitch_id) = user_response.id.parse::<i32>() else {
      return Err(EntityExtensionError::FailedToParseValue {
        value_name: "twitch user id",
        location: "try_from_twitch_user_response",
        value: user_response.id.to_string(),
      });
    };

    Ok(twitch_user::ActiveModel {
      login_name: Set(user_response.login),
      display_name: Set(user_response.display_name),
      twitch_id: Set(twitch_id),
      ..Default::default()
    })
  }
}
