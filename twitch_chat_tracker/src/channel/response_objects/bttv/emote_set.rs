#[derive(Debug, serde::Deserialize)]
pub struct BttvEmote {
  pub id: String,

  #[serde(rename = "code")]
  pub name: String,
}
