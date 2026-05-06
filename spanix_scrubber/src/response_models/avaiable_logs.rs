use reqwest::StatusCode;
use std::cmp::Ordering;
use twitch_chat_tracker::errors::AppError;

const LOG_QUERY_URL: &str =
  "https://logs.spanix.team/list?channel={channel_login}&user={user_login}";

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct AvailableLogs {
  #[serde(rename = "availableLogs")]
  pub logs: Vec<LogEntry>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct LogEntry {
  pub year: String,
  pub month: String,
}

impl AvailableLogs {
  pub async fn get_available_logs_for_user(
    user_login: &str,
    channel_login: &str,
  ) -> Result<Option<AvailableLogs>, AppError> {
    let reqwest_client = reqwest::Client::new();
    let url = Self::build_url(user_login, channel_login);

    let response = reqwest_client.get(url).send().await?;

    let status = response.status();

    if !status.is_success() {
      if status == StatusCode::NOT_FOUND {
        return Ok(None);
      }

      return Err(AppError::FailedResponse {
        location: "get user messages from spanix",
        code: status.as_u16(),
      });
    }

    response.json().await.map_err(Into::into).map(Some)
  }

  fn build_url(user_login: &str, channel_login: &str) -> String {
    LOG_QUERY_URL
      .replace("{channel_login}", channel_login)
      .replace("{user_login}", user_login)
  }

  pub fn remove_after_date(&mut self, remove_after_year: i32, remove_after_month: i32) {
    assert!(remove_after_month <= 12);

    self.logs.retain(|entry| {
      let year = entry.year.parse::<i32>().unwrap();
      let month = entry.month.parse::<i32>().unwrap();

      match year.cmp(&remove_after_year) {
        Ordering::Greater => false,
        Ordering::Less => true,
        Ordering::Equal => month <= remove_after_month,
      }
    });
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_remove_after_date_works() {
    let mut list = AvailableLogs {
      logs: vec![
        LogEntry {
          year: "2025".into(),
          month: "3".into(),
        },
        LogEntry {
          year: "2025".into(),
          month: "4".into(),
        },
        LogEntry {
          year: "2025".into(),
          month: "5".into(),
        },
      ],
    };
    let expected_list = AvailableLogs {
      logs: vec![LogEntry {
        year: "2025".into(),
        month: "3".into(),
      }],
    };

    list.remove_after_date(2025, 3);

    assert_eq!(list, expected_list);
  }
}
