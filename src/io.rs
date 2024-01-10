use std::collections::HashMap;

use crate::http::Response;
pub trait EnvIO: Send + Sync {
  fn get(&self, key: &str) -> Option<String>;
}

#[async_trait::async_trait]
pub trait HttpIO: Sync + Send {
  async fn execute(&self, request: reqwest::Request) -> anyhow::Result<Response<async_graphql::Value>> {
    self.execute_raw(request).await?.to_json()
  }
  async fn execute_raw(&self, request: reqwest::Request) -> anyhow::Result<Response<Vec<u8>>>;
}

#[async_trait::async_trait]
pub trait FileIO {
  async fn write<'a>(&'a self, file: &'a str, content: &'a [u8]) -> anyhow::Result<()>;
  async fn read<'a>(&'a self, file_path: &'a str) -> anyhow::Result<(String, String)>;
  async fn read_all<'a>(&'a self, file_paths: &'a [String]) -> anyhow::Result<Vec<(String, String)>>;
}

// TODO: rename to ConstEnv
pub struct Env {
  env: HashMap<String, String>,
}

impl EnvIO for Env {
  fn get(&self, key: &str) -> Option<String> {
    self.env.get(key).cloned()
  }
}

impl Env {
  pub fn init(map: HashMap<String, String>) -> Self {
    Self { env: map }
  }
}
