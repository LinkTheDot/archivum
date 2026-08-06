use serde::Deserialize;

pub fn deserialize_from_string<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
  T: std::str::FromStr,
  T::Err: std::fmt::Display,
  D: serde::Deserializer<'de>,
{
  String::deserialize(deserializer)?
    .parse()
    .map_err(serde::de::Error::custom)
}

pub fn deserialize_from_option_string<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
  T: std::str::FromStr,
  T::Err: std::fmt::Display,
  D: serde::Deserializer<'de>,
{
  Option::<String>::deserialize(deserializer)?
    .map(|s| s.parse().map_err(serde::de::Error::custom))
    .transpose()
}
