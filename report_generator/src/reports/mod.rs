pub mod basic_reports;
pub mod annual_chats;
pub mod chosen_report;
pub mod subathon_points;
pub mod subathon_reports;

#[derive(Debug, Default)]
pub struct Reports {
  reports: Vec<Report>,
}

#[derive(Debug)]
pub struct Report {
  pub name: &'static str,
  pub body: String,
}

impl Reports {
  pub fn get_reports(&self) -> &Vec<Report> {
    &self.reports
  }

  pub fn add_reports(&mut self, mut add_reports: Vec<Report>) {
    self.reports.append(&mut add_reports);
  }
}

impl Report {
  pub fn new(name: &'static str, body: String) -> Self {
    Self { name, body }
  }

  pub fn build_report_from_list(name: &'static str, reports: &[&str], join_string: &str) -> Self {
    let report_body = reports
      .iter()
      .filter(|t| !t.is_empty())
      .copied()
      .collect::<Vec<&str>>()
      .join(join_string);

    Self::new(name, report_body)
  }
}
