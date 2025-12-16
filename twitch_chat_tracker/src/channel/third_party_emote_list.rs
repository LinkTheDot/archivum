use crate::{
  channel::response_objects::{
    bttv::{global_response::BttvGlobalResponse, user_response::BttvUserResponse},
    emote_response::EmoteResponseList,
    franker_face_z::{
      global_response::FrankerFaceZGlobalResponse, user_response::FrankerFaceZUserResponse,
    },
    seven_tv::{global_response::SevenTvGlobalResponse, user_response::SevenTvUserResponse},
  },
  errors::AppError,
};
use app_config::AppConfig;
use entities::*;
use reqwest::StatusCode;
use sea_orm::DatabaseConnection;
use sea_orm_active_enums::ExternalService;
use std::collections::HashMap;
use url::Url;

const SEVEN_TV_API_URL: &str = "https://7tv.io/v3/";
const SEVEN_TV_GLOBAL_EMOTE_PATH: &str = "emote-sets/global";
const SEVEN_TV_USER_EMOTE_PATH: &str = "users/twitch/";

const BTTV_API_URL: &str = "https://api.betterttv.net/3/cached/";
const BTTV_GLOBAL_EMOTE_PATH: &str = "emotes/global";
const BTTV_USER_EMOTE_PATH: &str = "users/twitch/";

const FRANKER_FACE_Z_GLOBAL_EMOTE_URL: &str =
  "https://api.betterttv.net/3/cached/frankerfacez/emotes/global";
const FRANKER_FACE_Z_USER_EMOTE_URL: &str = "https://api.frankerfacez.com/v1/room/id/";

// -= Global Emote Lists =-
// 7TV:  https://7tv.io/v3/emote-sets/global
// BTTV: https://api.betterttv.net/3/cached/emotes/global
// FFZ:  https://api.betterttv.net/3/cached/frankerfacez/emotes/global
//
// -= User Emote Lists =-
// 7TV:  https://7tv.io/v3/users/twitch/578762718
// BTTV: https://api.betterttv.net/3/cached/users/twitch/578762718
// FFZ:  https://api.frankerfacez.com/v1/room/id/578762718
//
// -= Fetch Image Urls =-
// 7TV:  https://cdn.7tv.app/emote/{id}/4x.webp
// BTTV: https://cdn.betterttv.net/emote/{id}/3x.webp
// FFZ:  https://cdn.frankerfacez.com/emote/{id}/4
#[derive(Debug)]
pub struct EmoteList {
  channel_name: String,
  /// Key: emote_name | Value: EmoteModel
  emote_list: HashMap<String, emote::Model>,
}

impl EmoteList {
  pub const GLOBAL_NAME: &str = "GLOBAL";
  /// Conains the (name, id) for emotes
  pub const TEST_EMOTES: &[(&str, &str)] = &[
    ("glorp", "01H16FA16G0005EZED5J0EY7KN"),
    ("waaa", "01FTCXPJ200001E12995B12626"),
    ("glorpass", "01JAQC65ZG07ABT7PJ082ZTF9M"),
  ];

  pub fn get_empty(channel_name: String) -> Self {
    Self {
      channel_name,
      emote_list: HashMap::default(),
    }
  }

  pub async fn get_list(
    channel: &twitch_user::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<Self, AppError> {
    tracing::info!("Getting emote list for channel {:?}", channel);

    let emote_list = Self::get_full_emote_list(channel, database_connection).await?;

    Ok(Self {
      channel_name: channel.login_name.to_owned(),
      emote_list,
    })
  }

  pub async fn get_global_list(database_connection: &DatabaseConnection) -> Result<Self, AppError> {
    let mut global_emotes = EmoteResponseList::default();

    tracing::info!("Fetching global 7TV emotes.");
    let global_seven_tv_fetch_url =
      Url::parse(SEVEN_TV_API_URL)?.join(SEVEN_TV_GLOBAL_EMOTE_PATH)?;
    let global_seven_tv_emotes =
      Self::get_emote_response::<SevenTvGlobalResponse>(global_seven_tv_fetch_url).await?;

    tracing::info!("Fetching global BTTV emotes.");
    let global_bttv_fetch_url = Url::parse(BTTV_API_URL)?.join(BTTV_GLOBAL_EMOTE_PATH)?;
    let global_bttv_emotes =
      Self::get_emote_response::<BttvGlobalResponse>(global_bttv_fetch_url).await?;

    tracing::info!("Fetching global FrankerFaceZ emotes.");
    let global_franker_face_z_fetch_url = Url::parse(FRANKER_FACE_Z_GLOBAL_EMOTE_URL)?;
    let global_franker_face_z_emotes =
      Self::get_emote_response::<FrankerFaceZGlobalResponse>(global_franker_face_z_fetch_url)
        .await?;

    global_emotes.extend(global_seven_tv_emotes);
    global_emotes.extend(global_bttv_emotes);
    global_emotes.extend(global_franker_face_z_emotes);

    let global_emotes = global_emotes
      .batch_insert_emotes(database_connection)
      .await?;
    let global_emote_map: HashMap<String, emote::Model> = global_emotes
      .into_iter()
      .map(|emote| (emote.name.clone(), emote))
      .collect();

    Ok(Self {
      channel_name: Self::GLOBAL_NAME.to_string(),
      emote_list: global_emote_map,
    })
  }

  /// Returns the `emote_name | Emote` for the channel from 7tv, bttv, and frankerfacez.
  #[allow(unused)]
  async fn get_full_emote_list(
    channel: &twitch_user::Model,
    database_connection: &DatabaseConnection,
  ) -> Result<HashMap<String, emote::Model>, AppError> {
    let channel_twitch_id = channel.twitch_id.to_string();

    let mut emotes = EmoteResponseList::default();

    tracing::info!("Fetching 7TV emotes for `{}`", channel.login_name);
    let seven_tv_emotes = Self::seven_tv_emote_list(channel).await?;
    tracing::info!("Fetching BTTV emotes for `{}`", channel.login_name);
    let bttv_emotes = Self::bttv_emote_list(channel).await?;
    tracing::info!("Fetching FrankerFaceZ emotes for `{}`", channel.login_name);
    let franker_face_z_emotes = Self::franker_face_z_emote_list(channel).await?;

    emotes.extend(seven_tv_emotes);
    emotes.extend(bttv_emotes);
    emotes.extend(franker_face_z_emotes);

    let emotes = emotes.batch_insert_emotes(database_connection).await?;
    let emote_map: HashMap<String, emote::Model> = emotes
      .into_iter()
      .map(|emote| (emote.name.clone(), emote))
      .collect();

    Ok(emote_map)
  }

  async fn seven_tv_emote_list(
    channel: &twitch_user::Model,
  ) -> Result<EmoteResponseList, AppError> {
    let base_url = Url::parse(SEVEN_TV_API_URL)?;

    let user_seven_tv_fetch_url = base_url
      .join(SEVEN_TV_USER_EMOTE_PATH)?
      .join(&channel.twitch_id.to_string())?;
    let seven_tv_emote_list =
      Self::get_emote_response::<SevenTvUserResponse>(user_seven_tv_fetch_url).await?;

    Ok(seven_tv_emote_list)
  }

  async fn bttv_emote_list(channel: &twitch_user::Model) -> Result<EmoteResponseList, AppError> {
    let base_url = Url::parse(BTTV_API_URL)?;

    let user_fetch_url = base_url
      .join(BTTV_USER_EMOTE_PATH)?
      .join(&channel.twitch_id.to_string())?;
    let bttv_emotes = Self::get_emote_response::<BttvUserResponse>(user_fetch_url).await?;

    Ok(bttv_emotes)
  }

  async fn franker_face_z_emote_list(
    channel: &twitch_user::Model,
  ) -> Result<EmoteResponseList, AppError> {
    let user_fetch_url =
      Url::parse(FRANKER_FACE_Z_USER_EMOTE_URL)?.join(&channel.twitch_id.to_string())?;
    let franker_face_z_emotes =
      Self::get_emote_response::<FrankerFaceZUserResponse>(user_fetch_url).await?;

    Ok(franker_face_z_emotes)
  }

  /// Fetches an emote response from the given URL and builds it into the given type.
  ///
  /// Ignores any 404 error and returns an empty list if they occurr.
  async fn get_emote_response<ResponseType>(fetch_url: Url) -> Result<EmoteResponseList, AppError>
  where
    ResponseType: for<'de> serde::Deserialize<'de> + Into<EmoteResponseList>,
  {
    let reqwest_client = reqwest::Client::new();
    let response = reqwest_client.get(fetch_url).send().await?;

    let status = response.status();

    if !status.is_success() {
      // Not found implies the user did not setup an account with this service.
      // We don't care in this case because it means they had no emotes from here anyways.
      if status == StatusCode::NOT_FOUND {
        return Ok(EmoteResponseList::default());
      } else {
        return Err(AppError::FailedResponse {
          location: "get_third_party_emote_response",
          code: status.as_u16(),
        });
      }
    }

    let response_body = response.text().await?;

    let emote_response_list: EmoteResponseList =
      serde_json::from_str::<ResponseType>(&response_body)?.into();

    Ok(emote_response_list)
  }

  /// Returns the list of emotes defined by EmoteList::TEST_EMOTES for every channel under AppConfig::TEST_CHANNELS and Self::GLOBAL_NAME.
  ///
  /// None is returned if this method is called without the test flag set.
  pub fn get_test_list() -> Option<Vec<Self>> {
    if !cfg!(test) {
      return None;
    }

    let test_emotes: HashMap<String, emote::Model> = Self::TEST_EMOTES
      .iter()
      .enumerate()
      .map(|(iteration, (emote_name, emote_id))| {
        let emote = emote::Model {
          id: iteration as i32 + 1,
          external_id: emote_id.to_string(),
          name: emote_name.to_string(),
          external_service: ExternalService::SevenTv,
        };

        (emote_name.to_string(), emote)
      })
      // (emote_name.to_string(), emote_id.to_string()))
      .collect();
    let mut emote_lists = vec![];

    for channel_name in AppConfig::TEST_CHANNELS {
      emote_lists.push(EmoteList {
        channel_name: channel_name.to_string(),
        emote_list: test_emotes.clone(),
      })
    }

    emote_lists.push(EmoteList {
      channel_name: Self::GLOBAL_NAME.to_string(),
      emote_list: test_emotes,
    });

    Some(emote_lists)
  }

  /// Returns the combined list of 7tv, bttv, and frankerfacez emotes.
  ///
  /// Key: Name | Value: ID
  pub fn emote_list(&self) -> &HashMap<String, emote::Model> {
    &self.emote_list
  }

  pub fn contains(&self, value: &str) -> bool {
    self.emote_list.contains_key(value)
  }

  pub fn channel_name(&self) -> &str {
    &self.channel_name
  }

  pub fn get(&self, emote_name: &str) -> Option<&emote::Model> {
    self.emote_list.get(emote_name)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn create_test_emote(id: i32, name: &str, external_id: &str) -> emote::Model {
    emote::Model {
      id,
      name: name.to_string(),
      external_id: external_id.to_string(),
      external_service: ExternalService::SevenTv,
    }
  }

  fn create_emote_list_with_emotes(channel_name: &str, emotes: Vec<emote::Model>) -> EmoteList {
    let emote_list: HashMap<String, emote::Model> =
      emotes.into_iter().map(|e| (e.name.clone(), e)).collect();

    EmoteList {
      channel_name: channel_name.to_string(),
      emote_list,
    }
  }

  mod contains {
    use super::*;

    #[test]
    fn returns_true_for_existing_emote() {
      let emotes = vec![create_test_emote(1, "test_emote", "ext1")];
      let emote_list = create_emote_list_with_emotes("channel", emotes);

      assert!(emote_list.contains("test_emote"));
    }

    #[test]
    fn returns_false_for_non_existing_emote() {
      let emotes = vec![create_test_emote(1, "test_emote", "ext1")];
      let emote_list = create_emote_list_with_emotes("channel", emotes);

      assert!(!emote_list.contains("non_existent"));
    }

    #[test]
    fn is_case_sensitive() {
      let emotes = vec![create_test_emote(1, "TestEmote", "ext1")];
      let emote_list = create_emote_list_with_emotes("channel", emotes);

      assert!(emote_list.contains("TestEmote"));
      assert!(!emote_list.contains("testemote"));
      assert!(!emote_list.contains("TESTEMOTE"));
    }
  }

  mod get {
    use super::*;

    #[test]
    fn returns_some_for_existing_emote() {
      let emotes = vec![create_test_emote(1, "test_emote", "ext123")];
      let emote_list = create_emote_list_with_emotes("channel", emotes);

      let result = emote_list.get("test_emote");

      assert!(result.is_some());
      let emote = result.unwrap();
      assert_eq!(emote.name, "test_emote");
      assert_eq!(emote.external_id, "ext123");
      assert_eq!(emote.id, 1);
    }

    #[test]
    fn returns_none_for_non_existing_emote() {
      let emotes = vec![create_test_emote(1, "test_emote", "ext1")];
      let emote_list = create_emote_list_with_emotes("channel", emotes);

      assert!(emote_list.get("non_existent").is_none());
    }

    #[test]
    fn returns_correct_emote_from_multiple() {
      let emotes = vec![
        create_test_emote(1, "emote_a", "ext_a"),
        create_test_emote(2, "emote_b", "ext_b"),
        create_test_emote(3, "emote_c", "ext_c"),
      ];
      let emote_list = create_emote_list_with_emotes("channel", emotes);

      let result = emote_list.get("emote_b").unwrap();
      assert_eq!(result.id, 2);
      assert_eq!(result.external_id, "ext_b");
    }
  }
}
