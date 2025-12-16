use crate::prelude::*;
use crate::{
  errors::EntityExtensionError, twitch_user::response_objects::twitch_users::TwitchUserListResponse,
};
use app_config::AppConfig;
use app_config::secret_string::Secret;
pub use channel_identifier::*;
use entities::{
  twitch_user, twitch_user_name_change, twitch_user_unknown_user_association, unknown_user,
};
use sea_orm::*;
use strsim::jaro_winkler;
use url::Url;

pub mod channel_identifier;
mod get_many_by_identifier;
mod response_objects;

const HELIX_USER_QUERY_URL: &str = "https://api.twitch.tv/helix/users";
const JARO_NAME_SIMILARITY_THRESHOLD: f64 = 0.85;

pub trait TwitchUserExtensions {
  async fn get_by_identifier<S: AsRef<str>>(
    identifier: ChannelIdentifier<S>,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<twitch_user::Model>, EntityExtensionError>;
  async fn get_many_by_identifier<S: AsRef<str>>(
    identifiers: Vec<ChannelIdentifier<S>>,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<twitch_user::Model>, EntityExtensionError>;
  async fn get_list_by_incomplete_name<S: AsRef<str> + std::fmt::Debug>(
    identifier: ChannelIdentifier<S>,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<twitch_user::Model>, EntityExtensionError>;
  async fn get_or_set_by_name(
    login_name: &str,
    database_connection: &DatabaseConnection,
  ) -> Result<twitch_user::Model, EntityExtensionError>;
  async fn get_or_set_by_twitch_id(
    twitch_id: &str,
    database_connection: &DatabaseConnection,
  ) -> Result<twitch_user::Model, EntityExtensionError>;
  /// Queries Helix for every user passed in.
  async fn query_helix_for_channels_from_list<S: AsRef<str>>(
    channels: &[ChannelIdentifier<S>],
  ) -> Result<Vec<twitch_user::ActiveModel>, EntityExtensionError>;

  /// Takes a login name that might be within the database, and guesses the user using a levenshtein distance.
  ///
  /// Use this if [`get_or_set_by_name`](TwitchUserExtensions::get_or_set_by_name) fails on a name you expect to exist.
  async fn guess_name(
    guess_name: &str,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<twitch_user::Model>, EntityExtensionError>;
}

impl TwitchUserExtensions for twitch_user::Model {
  /// Retrieves the user based on the given identifier.
  ///
  /// Returns an error if the user was not found.
  async fn get_by_identifier<S: AsRef<str>>(
    identifier: ChannelIdentifier<S>,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<twitch_user::Model>, EntityExtensionError> {
    match identifier {
      ChannelIdentifier::Login(user_login) => {
        // -
        twitch_user::Entity::find()
          .filter(twitch_user::Column::LoginName.like(user_login.as_ref()))
          .one(database_connection)
          .await
          .map_err(Into::into)
      }
      ChannelIdentifier::TwitchID(twitch_id) => {
        // -
        twitch_user::Entity::find()
          .filter(twitch_user::Column::TwitchId.eq(twitch_id.as_ref()))
          .one(database_connection)
          .await
          .map_err(Into::into)
      }
    }
  }

  /// Retrieves the list of users from the given identifiers.
  ///
  /// Any users not already stored in the database are retrieved from Twitch's API.
  async fn get_many_by_identifier<S: AsRef<str>>(
    identifiers: Vec<ChannelIdentifier<S>>,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<twitch_user::Model>, EntityExtensionError> {
    get_many_by_identifier::get_many_by_identifier(identifiers, database_connection).await
  }

  /// Returns the list of users based on an incomplete login.
  /// For example, if you pass in a login of "fall", then the list of every user with "fall" in their name is returned.
  ///
  /// An error is returned if there are no matches.
  async fn get_list_by_incomplete_name<S: AsRef<str> + std::fmt::Debug>(
    identifier: ChannelIdentifier<S>,
    database_connection: &DatabaseConnection,
  ) -> Result<Vec<twitch_user::Model>, EntityExtensionError> {
    let user_list_result = match &identifier {
      ChannelIdentifier::Login(approximate_login) => {
        // -
        twitch_user::Entity::find()
          .filter(twitch_user::Column::LoginName.contains(approximate_login.as_ref()))
      }
      ChannelIdentifier::TwitchID(twitch_id) => {
        // -
        twitch_user::Entity::find().filter(twitch_user::Column::TwitchId.eq(twitch_id.as_ref()))
      }
    }
    .all(database_connection)
    .await
    .map_err(Into::into);

    if let Ok(user_list) = &user_list_result
      && user_list.is_empty()
    {
      return Err(EntityExtensionError::FailedToGetValue {
        value_name: "user",
        location: "get_list_by_incomplete_name",
        additional_data: format!("{:?}", identifier),
      });
    }

    user_list_result
  }

  /// Retrieves the user model from the database if it exists.
  /// Otherwise creates the user entry for the database and returns the resulting model.                 
  ///
  /// Also updates the user's name if it was changed since last check.
  async fn get_or_set_by_name(
    user_name: &str,
    database_connection: &DatabaseConnection,
  ) -> Result<twitch_user::Model, EntityExtensionError> {
    let user_condition = Condition::any()
      .add(twitch_user::Column::LoginName.eq(user_name))
      .add(twitch_user::Column::DisplayName.eq(user_name));
    let user_model = twitch_user::Entity::find()
      .filter(user_condition)
      .one(database_connection)
      .await?;

    if let Some(user_model) = user_model {
      return Ok(user_model);
    }

    let helix_channel =
      Self::query_helix_for_channels_from_list(&[ChannelIdentifier::Login(user_name)]).await?;
    let Some(helix_channel) = helix_channel.first().cloned() else {
      return Err(EntityExtensionError::FailedToQuery {
        value_name: "helix user data",
        location: "get or set twitch user by name",
        value: user_name.to_owned(),
      });
    };

    let ActiveValue::Set(twitch_id) = helix_channel.twitch_id else {
      return Err(EntityExtensionError::FailedToGetValue {
        value_name: "twitch id",
        location: "get or set twitch user by name",
        additional_data: user_name.to_string(),
      });
    };
    let maybe_model = twitch_user::Entity::find()
      .filter(twitch_user::Column::TwitchId.eq(twitch_id))
      .one(database_connection)
      .await?;

    if let Some(existing_model) = maybe_model {
      let user_model =
        check_for_name_change(existing_model, helix_channel, database_connection).await?;

      return Ok(user_model);
    } else {
      tracing::trace!("Found a new channel from Helix: {:?}", helix_channel);

      attempt_insert(helix_channel, database_connection).await
    }
  }

  /// Retrieves the user model from the database if it exists.
  /// Otherwise creates the user entry for the database and returns the resulting model.
  ///
  /// Also updates the user's name if it was changed since last check.
  async fn get_or_set_by_twitch_id(
    twitch_id: &str,
    database_connection: &DatabaseConnection,
  ) -> Result<twitch_user::Model, EntityExtensionError> {
    let user_model = twitch_user::Entity::find()
      .filter(twitch_user::Column::TwitchId.eq(twitch_id))
      .one(database_connection)
      .await?;

    // if cfg!(test) || cfg!(feature = "__test_hook") {
    //   return Ok(user_model.unwrap());
    // }

    if let Some(user_model) = user_model {
      return Ok(user_model);
    }

    let helix_channel =
      Self::query_helix_for_channels_from_list(&[ChannelIdentifier::TwitchID(twitch_id)]).await?;
    let Some(helix_channel) = helix_channel.first().cloned() else {
      return Err(EntityExtensionError::FailedToQuery {
        value_name: "helix user data",
        location: "get or set twitch user by twitch id",
        value: twitch_id.to_owned(),
      });
    };

    attempt_insert(helix_channel, database_connection).await
  }

  async fn query_helix_for_channels_from_list<S: AsRef<str>>(
    channels: &[ChannelIdentifier<S>],
  ) -> Result<Vec<twitch_user::ActiveModel>, EntityExtensionError> {
    if channels.is_empty() || cfg!(feature = "__test_hook") || cfg!(test) {
      return Ok(vec![]);
    }

    let mut query_url = Url::parse(HELIX_USER_QUERY_URL)?;
    let reqwest_client = reqwest::Client::new();

    {
      let mut query_pairs = query_url.query_pairs_mut();

      for channel_name in channels {
        match channel_name {
          ChannelIdentifier::Login(channel_name) => {
            query_pairs.append_pair("login", channel_name.as_ref());
          }
          ChannelIdentifier::TwitchID(twitch_id) => {
            query_pairs.append_pair("id", twitch_id.as_ref());
          }
        }
      }
    }

    let request = reqwest_client
      .get(query_url)
      .header(
        "Authorization",
        format!(
          "Bearer {}",
          Secret::read_secret_string(AppConfig::access_token().read_value())
        ),
      )
      .header(
        "Client-Id",
        Secret::read_secret_string(AppConfig::client_id().read_value()),
      );

    let response = request.send().await?;
    let response_body = response.text().await?;

    let response_list: TwitchUserListResponse = serde_json::from_str(&response_body)?;

    response_list
      .data
      .into_iter()
      .map(twitch_user::ActiveModel::try_from)
      .collect()
  }

  /// Takes a guessed name and compares it against all login and display names in the database.
  ///
  /// If the name matches close enough to one in the database, the model for it is returned.
  /// Otherwise None is returned.
  async fn guess_name(
    guess_name: &str,
    database_connection: &DatabaseConnection,
  ) -> Result<Option<twitch_user::Model>, EntityExtensionError> {
    let unknown_user =
      unknown_user::Model::get_or_set_by_name(guess_name, database_connection).await?;
    let maybe_association = unknown_user
      .get_associated_twich_user(database_connection)
      .await?;

    if let Some(associated_user) = maybe_association {
      return Ok(Some(associated_user));
    }

    let all_users = twitch_user::Entity::find().all(database_connection).await?;

    let maybe_user_match = all_users.into_iter().find(|user| {
      jaro_winkler(&user.login_name, guess_name) >= JARO_NAME_SIMILARITY_THRESHOLD
        || jaro_winkler(&user.display_name, guess_name) >= JARO_NAME_SIMILARITY_THRESHOLD
    });

    if let Some(matched_user) = &maybe_user_match {
      let _ = twitch_user_unknown_user_association::Model::get_or_set_connection(
        &unknown_user,
        matched_user,
        database_connection,
      )
      .await?;
    }

    Ok(maybe_user_match)
  }
}

/// Checks if the user changed their name or not. Adding a [`twitch_user_name_change`](crate::twitch_user_name_change::Model) and updating the existing entry.
///
/// Returns the user after updating.
///
/// If there was no change returnx the existing user back.
async fn check_for_name_change(
  existing_twitch_user: twitch_user::Model,
  helix_twitch_user: twitch_user::ActiveModel,
  database_connection: &DatabaseConnection,
) -> Result<twitch_user::Model, EntityExtensionError> {
  tracing::info!(
    "Updating user name change from {} to {:?}",
    existing_twitch_user.login_name,
    helix_twitch_user.login_name
  );

  if existing_twitch_user.login_name == *helix_twitch_user.login_name.as_ref() {
    return Ok(existing_twitch_user);
  }

  let name_change = twitch_user_name_change::ActiveModel {
    twitch_user_id: ActiveValue::Set(existing_twitch_user.id),
    previous_login_name: ActiveValue::Set(Some(existing_twitch_user.login_name.clone())),
    previous_display_name: ActiveValue::Set(Some(existing_twitch_user.display_name.clone())),
    new_login_name: helix_twitch_user.login_name.clone().into(),
    new_display_name: helix_twitch_user.display_name.clone().into(),
    ..Default::default()
  };

  let updated_twitch_user = twitch_user::ActiveModel {
    login_name: helix_twitch_user.login_name,
    display_name: helix_twitch_user.display_name,
    ..existing_twitch_user.into_active_model()
  };

  name_change.insert(database_connection).await?;

  updated_twitch_user
    .update(database_connection)
    .await
    .map_err(Into::into)
}

/// Attempts to insert the user into the database.
///
/// If there is a unique constraint violation, attempts to get the user again and returns the value.
async fn attempt_insert(
  helix_channel: twitch_user::ActiveModel,
  database_connection: &DatabaseConnection,
) -> Result<twitch_user::Model, EntityExtensionError> {
  let ActiveValue::Set(twitch_id) = helix_channel.twitch_id else {
    return Err(EntityExtensionError::FailedToGetValue {
      value_name: "twitch id",
      location: "attempt insert",
      additional_data: format!("{:?}", helix_channel),
    });
  };
  let result = helix_channel.insert(database_connection).await;

  // Checking if there was a race condition where another process is inserting at the same time.
  if let Err(error) = &result
    && let Some(SqlErr::UniqueConstraintViolation(_)) = error.sql_err()
  {
    let user_model_result = twitch_user::Entity::find()
      .filter(twitch_user::Column::TwitchId.eq(twitch_id))
      .one(database_connection)
      .await?;

    if let Some(user_model) = user_model_result {
      return Ok(user_model);
    } else {
      return Err(EntityExtensionError::FailedToQuery {
        value_name: "twitch user",
        location: "attempt insert",
        value: twitch_id.to_string(),
      });
    }
  }

  result.map_err(Into::into)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn get_by_identifier_method_works_with_string_twitch_ids() {
    let user_model = twitch_user::Model {
      id: 1,
      twitch_id: 578762718,
      login_name: "fallenshadow".into(),
      display_name: "fallenshadow".into(),
    };
    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_query_results([vec![user_model.clone()]])
      .into_connection();

    let twitch_id = ChannelIdentifier::TwitchID("578762718");

    let result = twitch_user::Model::get_by_identifier(twitch_id, &mock_database)
      .await
      .unwrap();

    assert_eq!(result, Some(user_model));
  }

  #[tokio::test]
  async fn get_by_identifier_method_works_with_logins() {
    let user_model = twitch_user::Model {
      id: 1,
      twitch_id: 578762718,
      login_name: "fallenshadow".into(),
      display_name: "fallenshadow".into(),
    };
    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_query_results([vec![user_model.clone()]])
      .into_connection();

    let login = ChannelIdentifier::Login("fallenshadow");

    let result = twitch_user::Model::get_by_identifier(login, &mock_database)
      .await
      .unwrap();

    assert_eq!(result, Some(user_model));
  }

  #[tokio::test]
  async fn get_list_by_incomplete_name_works_for_list() {
    let user_models = vec![
      twitch_user::Model {
        id: 1,
        twitch_id: 578762718,
        login_name: "fallenshadow".into(),
        display_name: "fallenshadow".into(),
      },
      twitch_user::Model {
        id: 2,
        twitch_id: 795025340,
        login_name: "shadowchama".into(),
        display_name: "shadowchama".into(),
      },
    ];
    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_query_results([user_models.clone()])
      .into_connection();

    let incomplete_login = ChannelIdentifier::Login("shadow");

    let result = twitch_user::Model::get_list_by_incomplete_name(incomplete_login, &mock_database)
      .await
      .unwrap();

    assert_eq!(result, user_models);
  }

  #[tokio::test]
  async fn get_list_by_incomplete_name_returns_empty_list_with_no_results() {
    let user_models: Vec<twitch_user::Model> = vec![];
    let mock_database = MockDatabase::new(DatabaseBackend::MySql)
      .append_query_results([user_models])
      .into_connection();

    let incomplete_login = ChannelIdentifier::Login("missing_user");

    let result =
      twitch_user::Model::get_list_by_incomplete_name(incomplete_login, &mock_database).await;

    assert!(
      matches!(result, Err(EntityExtensionError::FailedToGetValue { .. })),
      "{:?}",
      result
    );
  }
}
