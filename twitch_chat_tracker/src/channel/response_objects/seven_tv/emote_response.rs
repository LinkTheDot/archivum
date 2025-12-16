#[derive(Debug, serde::Deserialize)]
pub struct SevenTvEmoteSet {
  pub emotes: Vec<SevenTvEmote>,
}

#[derive(Debug, serde::Deserialize)]
pub struct SevenTvEmote {
  pub id: String,
  pub name: String,
}
