use crate::{conditions::query_conditions::AppQueryConditions, errors::AppError};
use entities::{twitch_user, user_timeout};
use human_time::ToHumanTimeString;
use sea_orm::*;
use std::time::Duration;

const TIMEOUT_HEADER: &str = "= Timeouts =";
const BANS_HEADER: &str = "= Bans =";

pub async fn get_timeouts_table(
  query_conditions: &AppQueryConditions,
  database_connection: &DatabaseConnection,
) -> Result<String, AppError> {
  tracing::info!("Building timeouts table");

  let (mut banned_users, timedout_users) =
    get_timeouts(query_conditions, database_connection).await?;

  let timedout_users_string = generate_timedout_users_string(timedout_users, &mut banned_users);
  let banned_users_string = generate_banned_users_string(banned_users);

  let mut timeout_table_string = String::new();

  if !timedout_users_string.is_empty() {
    timeout_table_string.push_str(&format!("{TIMEOUT_HEADER}\n{timedout_users_string}\n"));
  }

  if !banned_users_string.is_empty() {
    if !timedout_users_string.is_empty() {
      timeout_table_string.push('\n')
    }

    timeout_table_string.push_str(&format!("{BANS_HEADER}\n{banned_users_string}\n"));
  }

  Ok(timeout_table_string)
}

fn generate_timedout_users_string(
  timedout_users: Vec<(user_timeout::Model, twitch_user::Model)>,
  banned_users: &mut Vec<(user_timeout::Model, twitch_user::Model)>,
) -> String {
  tracing::info!("Building timed-out users string.");

  timedout_users
    .into_iter()
    .filter_map(|(timeout, user)| {
      let Some(timeout_duration) = timeout.duration else {
        tracing::warn!(
          "Ban found in timeout list. Moving contents. Timeout ID: {:?}",
          timeout.id
        );
        banned_users.push((timeout, user));
        return None;
      };

      let timeout_string = format!(
        "{} - {}",
        user.login_name,
        Duration::from_secs(timeout_duration as u64).to_human_time_string()
      );

      Some(timeout_string)
    })
    .collect::<Vec<String>>()
    .join("\n")
}

fn generate_banned_users_string(
  banned_users: Vec<(user_timeout::Model, twitch_user::Model)>,
) -> String {
  tracing::info!("Building banned users string.");

  banned_users
    .into_iter()
    .map(|(_timeout, user)| user.login_name)
    .collect::<Vec<String>>()
    .join("\n")
}

async fn get_timeouts(
  query_conditions: &AppQueryConditions,
  database_connection: &DatabaseConnection,
) -> Result<
  (
    Vec<(user_timeout::Model, twitch_user::Model)>,
    Vec<(user_timeout::Model, twitch_user::Model)>,
  ),
  AppError,
> {
  tracing::info!("Getting all timeouts.");

  let timeouts = user_timeout::Entity::find()
    .join(
      JoinType::LeftJoin,
      user_timeout::Relation::TwitchUser1.def(),
    )
    .filter(query_conditions.timeouts().clone())
    .select_also(twitch_user::Entity)
    .all(database_connection)
    .await?;

  tracing::info!("Filtering missing users.");

  let timeouts: Vec<(user_timeout::Model, twitch_user::Model)> = timeouts
    .into_iter()
    .filter_map(|(timeout, maybe_user)| {
      let Some(user) = maybe_user else {
        tracing::error!(
          "failed to find user for timeout event of ID `{}`",
          timeout.id
        );
        return None;
      };

      Some((timeout, user))
    })
    .collect();

  tracing::info!("Separating bans from timeouts.");

  Ok(
    timeouts
      .into_iter()
      .partition(|(timeout, _user)| timeout.is_permanent == 1),
  )
}
