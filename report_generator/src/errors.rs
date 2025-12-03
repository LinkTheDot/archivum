use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
  #[error("{}", .0)]
  SeaOrmDbError(#[from] sea_orm::error::DbErr),

  #[error("{}", .0)]
  IoError(#[from] std::io::Error),

  #[error("{}", .0)]
  FromUtf8Error(#[from] std::string::FromUtf8Error),

  #[error("{}", .0)]
  ReqwestError(#[from] reqwest::Error),

  #[error("{}", .0)]
  SerdeError(#[from] serde_json::Error),

  #[error("{}", .0)]
  TeraError(#[from] tera::Error),

  #[error("{}", .0)]
  EntityExtensionError(#[from] entity_extensions::errors::EntityExtensionError),

  #[error("Failed to generate a pastebin. Reason: {:?}", .0)]
  IncorrectPastebinResponse(String),

  #[error(
    "Could not convert currency rates. Missing API key for https://app.exchangerate-api.com/"
  )]
  MissingEchangeRateApiKey,

  #[error("Received an unknown response body structure when querying. Body location: {:?}", .0)]
  UnknownResponseBody(&'static str),

  #[error("Attempted to retrieve currency exchange rates, but received an errored response. Error code: {:?}", .0)]
  FailedToRetrieveCurrenyExchangeRates(StatusCode),

  #[error(
    "Failed to convert currency from {} to {} because {} didn't exist.",
    from,
    to,
    to
  )]
  FailedToFindCurrencyValueInConversionRates { from: String, to: String },

  #[error("Failed to convert json number. Value: {:?}", .0)]
  FailedToConvertJsonNumber(serde_json::Number),

  #[error("Attempted to generate a report for donation rankings with an invalid month of {:?}", .0)]
  InvalidMonthValue(i32),

  #[error("Found no donations between given date range {start_date}-{end_date}")]
  NoDonationsRankings {
    start_date: chrono::DateTime<chrono::Utc>,
    end_date: chrono::DateTime<chrono::Utc>,
  },

  #[error("Could not find stream by ID {:?}", .0)]
  FailedToFindStream(i32),

  #[error("Attempted to upload to pastebin without an API key.")]
  MissingPastebinApiKey,

  #[error("Invalid query date range conditions. start: {} | end: {}", start, end)]
  InvalidQueryDateConditions { start: i32, end: i32 },

  #[error("Failed to render a template because it did not exist. Template name: {template_name}")]
  MissingTeraTemplate { template_name: &'static str },

  #[error("Failed to build conditions because the passed in end time `{end_time:?}` is older than the start time `{start_time:?}`.")]
  EndTimeIsOlderThanStartTime {
    start_time: chrono::DateTime<chrono::Utc>,
    end_time: chrono::DateTime<chrono::Utc>,
  },

  #[error("Tried to generate subathon report without a subathon start time.")]
  MissingSubathonStartTime,
}
