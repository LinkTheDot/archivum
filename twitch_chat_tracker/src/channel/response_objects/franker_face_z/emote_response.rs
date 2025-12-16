#[derive(Debug, serde::Deserialize)]
pub struct FrankerFaceZEmote {
  pub id: i32,

  #[serde(alias = "code")]
  pub name: String,
}
