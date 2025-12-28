#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchmarkName(pub &'static str);

impl BenchmarkName {
  pub fn new(name: &'static str) -> Self {
    Self(name)
  }
}

impl std::ops::Deref for BenchmarkName {
  type Target = str;

  fn deref(&self) -> &Self::Target {
    self.0
  }
}

impl std::fmt::Display for BenchmarkName {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(formatter, "{}", self.0)
  }
}
