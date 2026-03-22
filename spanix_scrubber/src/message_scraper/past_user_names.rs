/// A snapshot of what a user's login and display names were.
#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) struct UserName {
  pub login_name: String,
  pub display_name: String,
}

pub(super) struct PastUserNames(pub UserName);
