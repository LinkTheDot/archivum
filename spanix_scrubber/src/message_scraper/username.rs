/// A snapshot of what a user's login and display names were.
use entities::*;
#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) struct UserName {
  pub login_name: String,
  pub display_name: String,
}

#[derive(Default)]
pub(super) struct MaybeUserName {
  pub login_name: Option<String>,
  pub display_name: Option<String>,
}

impl UserName {
  /// If both values of MaybeUserName are None, None is returned.
  pub fn from_maybe_username(
    maybe_username: MaybeUserName,
    user: &twitch_user::Model,
  ) -> Option<Self> {
    if maybe_username.login_name.is_none() && maybe_username.display_name.is_none() {
      return None;
    }

    let login_name = maybe_username.login_name.unwrap_or(user.login_name.clone());
    let display_name = maybe_username
      .display_name
      .unwrap_or(user.display_name.clone());

    Some(Self {
      login_name,
      display_name,
    })
  }
}
