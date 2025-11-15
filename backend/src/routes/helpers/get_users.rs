use entities::twitch_user;
use sea_orm::*;

use crate::error::AppError;

pub trait GetUsers {
  fn get_login(&self) -> Option<&str> {
    None
  }

  fn get_twitch_id(&self) -> Option<&str> {
    None
  }

  /// This method expectes a string of names separated by commas like so: "name,name1,name2"
  fn get_many_logins(&self) -> Option<&str> {
    None
  }

  /// This method expectes a string of ids separated by commas like so: "111,222,333"
  fn get_many_twitch_ids(&self) -> Option<&str> {
    None
  }

  fn get_user_query(&self) -> Result<Select<twitch_user::Entity>, AppError> {
    self.get_maybe_user_query().ok_or(AppError::NoQueryParameterFound)
  }

  fn get_maybe_user_query(&self) -> Option<Select<twitch_user::Entity>> {
    if let Some(user_login) = self.get_login() {
      return Some(
        twitch_user::Entity::find().filter(twitch_user::Column::LoginName.contains(user_login)),
      );
    }

    if let Some(twitch_id) = self.get_twitch_id() {
      return Some(twitch_user::Entity::find().filter(twitch_user::Column::TwitchId.eq(twitch_id)));
    }

    if let Some(logins_string) = self.get_many_logins() {
      let logins: Vec<&str> = logins_string.split(',').collect();

      return Some(twitch_user::Entity::find().filter(twitch_user::Column::LoginName.is_in(logins)));
    }

    if let Some(twitch_ids) = self.get_many_twitch_ids() {
      let twitch_ids: Vec<&str> = twitch_ids.split(',').collect();

      return Some(
        twitch_user::Entity::find().filter(twitch_user::Column::TwitchId.is_in(twitch_ids)),
      );
    }

    None
  }

  fn get_missing_user_error(&self) -> AppError {
    if let Some(user_login) = self.get_login() {
      return AppError::CouldNotFindUserByLoginName {
        login: user_login.to_string(),
      };
    }

    if let Some(twitch_id) = self.get_twitch_id() {
      return AppError::CouldNotFindUserByTwitchId {
        user_id: twitch_id.to_string(),
      };
    }

    if let Some(logins_string) = self.get_many_logins() {
      return AppError::CouldNotFindUserByLoginName {
        login: logins_string.to_string(),
      };
    }

    if let Some(twitch_ids) = self.get_many_twitch_ids() {
      return AppError::CouldNotFindUserByTwitchId {
        user_id: twitch_ids.to_string(),
      };
    }

    AppError::NoQueryParameterFound
  }
}
