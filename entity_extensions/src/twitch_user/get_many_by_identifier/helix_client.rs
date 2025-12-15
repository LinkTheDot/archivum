use super::super::*;
use crate::errors::EntityExtensionError;
use entities::twitch_user;

pub trait HelixClient {
  async fn query_channels(
    &self,
    identifiers: &[ChannelIdentifier<&str>],
  ) -> Result<Vec<twitch_user::ActiveModel>, EntityExtensionError>;
}

pub struct RealHelixClient;

impl HelixClient for RealHelixClient {
  async fn query_channels(
    &self,
    identifiers: &[ChannelIdentifier<&str>],
  ) -> Result<Vec<twitch_user::ActiveModel>, EntityExtensionError> {
    twitch_user::Model::query_helix_for_channels_from_list(identifiers).await
  }
}

#[cfg(test)]
pub mod test_utils {
  use super::*;
  use std::collections::HashMap;

  pub struct MockHelixClient {
    responses: HashMap<String, twitch_user::ActiveModel>,
  }

  impl MockHelixClient {
    pub fn new() -> Self {
      Self {
        responses: HashMap::new(),
      }
    }

    pub fn with_user(mut self, identifier: &str, user: twitch_user::ActiveModel) -> Self {
      self.responses.insert(identifier.to_string(), user);
      self
    }
  }

  impl HelixClient for MockHelixClient {
    async fn query_channels(
      &self,
      identifiers: &[ChannelIdentifier<&str>],
    ) -> Result<Vec<twitch_user::ActiveModel>, EntityExtensionError> {
      let mut results = Vec::new();

      for identifier in identifiers {
        let key = identifier.to_str();

        if let Some(user) = self.responses.get(key) {
          results.push(user.clone());
        }
      }

      Ok(results)
    }
  }
}
