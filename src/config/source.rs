use thiserror::Error;

pub enum Source {
  Json,
  Yml,
  GraphQL,
}

const JSON_EXT: &str = "json";
const YML_EXT: &str = "yml";
const GRAPHQL_EXT: &str = "graphql";
const ALL: [Source; 3] = [Source::Json, Source::Yml, Source::GraphQL];

#[derive(Debug, Error)]
#[error("Unsupported file extension: {0}")]
pub struct UnsupportedFileFormat(String);

impl Source {
  pub fn ext(&self) -> &'static str {
    match self {
      Source::Json => JSON_EXT,
      Source::Yml => YML_EXT,
      Source::GraphQL => GRAPHQL_EXT,
    }
  }

  fn ends_with(&self, file: &str) -> bool {
    file.ends_with(&format!(".{}", self.ext()))
  }

  pub fn detect(name: &str) -> Result<Source, UnsupportedFileFormat> {
    ALL
      .into_iter()
      .find(|format| format.ends_with(name))
      .ok_or(UnsupportedFileFormat(name.to_string()))
  }
  pub fn detect_content_type(content_type: &str) -> Result<Source, UnsupportedFileFormat> {
    let content_type = content_type.split('/').last().unwrap();
    ALL
      .into_iter()
      .find(|format| format.ext().eq(content_type))
      .ok_or(UnsupportedFileFormat(content_type.to_string()))
  }
}
