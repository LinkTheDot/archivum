use super::*;
use crate::errors::EntityExtensionError;
use entities::twitch_user;
use helix_client::*;
use sea_orm::sea_query::OnConflict;

mod helix_client;

pub(super) async fn get_many_by_identifier<S: AsRef<str>>(
  identifiers: Vec<ChannelIdentifier<S>>,
  database_connection: &DatabaseConnection,
) -> Result<Vec<twitch_user::Model>, EntityExtensionError> {
  get_many_by_identifier_with_client(identifiers, database_connection, &RealHelixClient).await
}

pub async fn get_many_by_identifier_with_client<S: AsRef<str>, C: HelixClient>(
  identifiers: Vec<ChannelIdentifier<S>>,
  database_connection: &DatabaseConnection,
  helix_client: &C,
) -> Result<Vec<twitch_user::Model>, EntityExtensionError> {
  if identifiers.is_empty() {
    return Ok(vec![]);
  }

  let (login_names, twitch_ids) = separate_identifiers_by_type(&identifiers);
  let existing_users =
    query_users_by_identifiers(&login_names, &twitch_ids, database_connection).await?;
  let missing_identifiers = find_missing_identifiers(&identifiers, &existing_users);

  if missing_identifiers.is_empty() {
    return Ok(existing_users);
  }

  insert_missing_users_from_helix(&missing_identifiers, database_connection, helix_client).await?;

  let (missing_login_names, missing_twitch_ids) =
    separate_identifiers_by_type(&missing_identifiers);
  let newly_inserted_users = query_users_by_identifiers(
    &missing_login_names,
    &missing_twitch_ids,
    database_connection,
  )
  .await?;

  let mut all_users = existing_users;
  all_users.extend(newly_inserted_users);

  Ok(all_users)
}

fn separate_identifiers_by_type<S: AsRef<str>>(
  identifiers: &[ChannelIdentifier<S>],
) -> (Vec<&str>, Vec<&str>) {
  let login_names: Vec<&str> = identifiers
    .iter()
    .filter_map(|id| match id {
      ChannelIdentifier::Login(login) => Some(login.as_ref()),
      _ => None,
    })
    .collect();

  let twitch_ids: Vec<&str> = identifiers
    .iter()
    .filter_map(|id| match id {
      ChannelIdentifier::TwitchID(id) => Some(id.as_ref()),
      _ => None,
    })
    .collect();

  (login_names, twitch_ids)
}

async fn query_users_by_identifiers(
  login_names: &[&str],
  twitch_ids: &[&str],
  database_connection: &DatabaseConnection,
) -> Result<Vec<twitch_user::Model>, EntityExtensionError> {
  let mut condition = Condition::any();

  if !login_names.is_empty() {
    condition = condition.add(twitch_user::Column::LoginName.is_in(login_names.iter().copied()));
  }

  if !twitch_ids.is_empty() {
    condition = condition.add(twitch_user::Column::TwitchId.is_in(twitch_ids.iter().copied()));
  }

  twitch_user::Entity::find()
    .filter(condition)
    .all(database_connection)
    .await
    .map_err(Into::into)
}

fn find_missing_identifiers<'a, S: AsRef<str>>(
  identifiers: &'a [ChannelIdentifier<S>],
  existing_users: &[twitch_user::Model],
) -> Vec<ChannelIdentifier<&'a str>> {
  identifiers
    .iter()
    .filter(|identifier| !identifier_exists_in_users(identifier, existing_users))
    .map(|identifier| match identifier {
      ChannelIdentifier::Login(login) => ChannelIdentifier::Login(login.as_ref()),
      ChannelIdentifier::TwitchID(id) => ChannelIdentifier::TwitchID(id.as_ref()),
    })
    .collect()
}

fn identifier_exists_in_users<S: AsRef<str>>(
  identifier: &ChannelIdentifier<S>,
  users: &[twitch_user::Model],
) -> bool {
  match identifier {
    ChannelIdentifier::Login(login) => users
      .iter()
      .any(|user| user.login_name.eq_ignore_ascii_case(login.as_ref())),
    ChannelIdentifier::TwitchID(id) => users
      .iter()
      .any(|user| user.twitch_id.to_string() == id.as_ref()),
  }
}

async fn insert_missing_users_from_helix<C: HelixClient>(
  missing_identifiers: &[ChannelIdentifier<&str>],
  database_connection: &DatabaseConnection,
  helix_client: &C,
) -> Result<(), EntityExtensionError> {
  let helix_users = helix_client.query_channels(missing_identifiers).await?;

  if helix_users.is_empty() {
    return Ok(());
  }

  twitch_user::Entity::insert_many(helix_users)
    .on_conflict(
      OnConflict::column(twitch_user::Column::TwitchId)
        .do_nothing()
        .to_owned(),
    )
    .exec(database_connection)
    .await?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::helix_client::test_utils::*;
  use super::*;
  use sea_orm::{ActiveValue, DatabaseBackend, MockDatabase, MockExecResult};

  #[tokio::test]
  async fn test_get_many_returns_existing_users() {
    let user1 = twitch_user::Model {
      id: 1,
      twitch_id: 123,
      login_name: "user1".to_string(),
      display_name: "User1".to_string(),
    };

    let user2 = twitch_user::Model {
      id: 2,
      twitch_id: 456,
      login_name: "user2".to_string(),
      display_name: "User2".to_string(),
    };

    let mock_db = MockDatabase::new(DatabaseBackend::MySql)
      .append_query_results([vec![user1.clone(), user2.clone()]])
      .into_connection();

    let identifiers = vec![
      ChannelIdentifier::Login("user1"),
      ChannelIdentifier::Login("user2"),
    ];

    let mock_helix = MockHelixClient::new();

    let result = get_many_by_identifier_with_client(identifiers, &mock_db, &mock_helix)
      .await
      .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].login_name, "user1");
    assert_eq!(result[1].login_name, "user2");
  }

  #[tokio::test]
  async fn test_get_many_fetches_missing_users_from_helix() {
    let existing_user = twitch_user::Model {
      id: 1,
      twitch_id: 123,
      login_name: "existing".to_string(),
      display_name: "Existing".to_string(),
    };

    let new_user = twitch_user::Model {
      id: 2,
      twitch_id: 789,
      login_name: "newuser".to_string(),
      display_name: "NewUser".to_string(),
    };

    let mock_db = MockDatabase::new(DatabaseBackend::MySql)
      .append_query_results([vec![existing_user.clone()]])
      .append_exec_results([MockExecResult {
        last_insert_id: 2,
        rows_affected: 1,
      }])
      .append_query_results([vec![new_user.clone()]])
      .into_connection();

    let identifiers = vec![
      ChannelIdentifier::Login("existing"),
      ChannelIdentifier::Login("newuser"),
    ];

    let mock_helix = MockHelixClient::new().with_user(
      "newuser",
      twitch_user::ActiveModel {
        id: ActiveValue::NotSet,
        twitch_id: ActiveValue::Set(789),
        login_name: ActiveValue::Set("newuser".to_string()),
        display_name: ActiveValue::Set("NewUser".to_string()),
      },
    );

    let result = get_many_by_identifier_with_client(identifiers, &mock_db, &mock_helix)
      .await
      .unwrap();

    assert_eq!(result.len(), 2);
  }
}
