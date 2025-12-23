use crate::{clap::Args, errors::AppError};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug)]
pub struct TemplateRenderer {
  template_context: tera::Context,
  tera: tera::Tera,

  existing_template_names: HashSet<&'static str>,
}

impl TemplateRenderer {
  pub fn new() -> Self {
    let template_context = tera::Context::default();
    let tera = tera::Tera::default();

    Self {
      template_context,
      tera,

      existing_template_names: HashSet::new(),
    }
  }

  pub fn add_context<T: serde::Serialize>(&mut self, name: &'static str, context: &T) {
    self.template_context.insert(name, context);
  }

  pub fn add_template(
    &mut self,
    template_name: &'static str,
    template_contents: &str,
  ) -> Result<(), AppError> {
    let result = self.tera.add_raw_template(template_name, template_contents);

    if result.is_ok() {
      self.existing_template_names.insert(template_name);
    }

    result.map_err(Into::into)
  }

  pub fn add_many_templates(
    &mut self,
    names_and_templates: Vec<(&'static str, String)>,
  ) -> Result<(), AppError> {
    let names: Vec<&'static str> = names_and_templates
      .iter()
      .map(|(name, _)| name)
      .cloned()
      .collect();
    let result = self.tera.add_raw_templates(names_and_templates);

    if result.is_ok() {
      self.existing_template_names.extend(names.iter());
    }

    result.map_err(Into::into)
  }

  pub async fn add_template_from_file<P: AsRef<Path>>(
    &mut self,
    template_name: &'static str,
    template_path: P,
  ) -> Result<(), AppError> {
    let template_file_contents = fs::read_to_string(template_path).await?;

    self.add_template(template_name, &template_file_contents)
  }

  /// Adds the template for the given template file name defined in the template file directory.
  ///
  /// The default directory is "report_generator/template_files", but can be changed with the `--template-dir-path` flag.
  pub async fn add_template_from_template_file_name(
    &mut self,
    template_name: &'static str,
    template_file_name: &str,
  ) -> Result<(), AppError> {
    let template_path = {
      let mut template_path = PathBuf::from(Args::template_dir_path());
      template_path.push(template_file_name);

      template_path
    };

    let template_file_contents = fs::read_to_string(template_path).await?;

    self.add_template(template_name, &template_file_contents)
  }

  pub async fn add_many_templates_from_files<P: AsRef<Path>>(
    &mut self,
    names_and_template_paths: &[(&'static str, P)],
  ) -> Result<(), AppError> {
    let mut names_and_templates = vec![];

    for (name, template_path) in names_and_template_paths {
      let template_file_contents = fs::read_to_string(template_path).await?;

      names_and_templates.push((*name, template_file_contents));
    }

    self.add_many_templates(names_and_templates)
  }

  pub fn render(&self, template_name: &'static str) -> Result<String, AppError> {
    if !self.existing_template_names.contains(template_name) {
      return Err(AppError::MissingTeraTemplate { template_name });
    }

    self
      .tera
      .render(template_name, &self.template_context)
      .map_err(Into::into)
  }
}

impl Default for TemplateRenderer {
  fn default() -> Self {
    TemplateRenderer::new()
  }
}
