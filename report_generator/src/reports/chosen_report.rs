use std::str::FromStr;

#[derive(Debug, Default, Clone, Copy)]
pub enum ChosenReport {
  #[default]
  Basic,
  Subathon,
  CalculateSubathonPoints,
  AnnualMessages,
  Overall,
}

impl FromStr for ChosenReport {
  type Err = String;

  fn from_str(value: &str) -> Result<Self, Self::Err> {
    match value.to_lowercase().trim() {
      "basic" => Ok(Self::Basic),
      "subathon" => Ok(Self::Subathon),
      "calculate_subathon_points" => Ok(Self::CalculateSubathonPoints),
      "annual_messages" => Ok(Self::AnnualMessages),
      "overall" => Ok(Self::Overall),
      _ => Err(format!("Invalid variant: {}", value)),
    }
  }
}
