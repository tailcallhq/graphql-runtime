use std::sync::Arc;

use anyhow::anyhow;
use tailcall::io::FileIO;
use worker::Env;

use crate::r2_address::R2Address;
use crate::to_anyhow;

#[derive(Clone)]
pub struct CloudflareFileIO {
  env: Arc<Env>,
}
unsafe impl Send for CloudflareFileIO {}
unsafe impl Sync for CloudflareFileIO {}

impl CloudflareFileIO {
  pub fn init(env: Arc<Env>) -> Self {
    CloudflareFileIO { env }
  }
}

impl CloudflareFileIO {
  async fn bucket(&self, r2: &R2Address) -> anyhow::Result<worker::Bucket> {
    Ok(self.env.bucket(&r2.bucket).map_err(|e| anyhow!(e.to_string()))?)
  }

  async fn get(&self, r2: &R2Address) -> anyhow::Result<String> {
    let bucket = self.bucket(&r2).await.map_err(to_anyhow)?;
    let maybe_object = bucket.get(&r2.path).execute().await.map_err(to_anyhow)?;
    let object = maybe_object.ok_or(anyhow!("File {} was not found in bucket: {}", r2.path, r2.bucket))?;

    let body = match object.body() {
      Some(body) => body.text().await.map_err(to_anyhow),
      None => Ok("".to_string()),
    };
    body
  }

  async fn put(&self, r2: &R2Address, value: Vec<u8>) -> anyhow::Result<()> {
    let bucket = self.bucket(&r2).await.map_err(to_anyhow)?;
    bucket.put(&r2.path, value).execute().await.map_err(to_anyhow)?;
    Ok(())
  }
}

#[async_trait::async_trait]
impl FileIO for CloudflareFileIO {
  async fn write<'a>(&'a self, file_path: &'a str, content: &'a [u8]) -> anyhow::Result<()> {
    let io = self.clone();
    let file_path = file_path.to_string();
    let content = content.to_vec();
    async_std::task::spawn_local(async move {
      let r2 = R2Address::from_string(file_path)?;
      io.put(&r2, content).await.map_err(to_anyhow)?;
      anyhow::Ok(())
    })
    .await?;
    Ok(())
  }

  async fn read<'a>(&'a self, file_path: &'a str) -> anyhow::Result<String> {
    let file_path = file_path.to_string();
    let io = self.clone();
    async_std::task::spawn_local(async move {
      let r2 = R2Address::from_string(file_path)?;
      let content = io.get(&r2).await.map_err(to_anyhow)?;
      anyhow::Ok(content)
    })
    .await
  }

  async fn read_all<'a>(&'a self, file_paths: &'a [String]) -> anyhow::Result<Vec<(String, String)>> {
    let file_paths = file_paths.to_vec();
    let io = self.clone();
    async_std::task::spawn_local(async move {
      let mut vec = Vec::new();
      // TODO: read files in parallel
      for file in file_paths {
        let content = io.read(&file).await.map_err(to_anyhow)?;
        vec.push((content, file.to_string()));
      }
      anyhow::Ok(vec)
    })
    .await
  }
}

/*async fn internal_shi(io: CloudflareFileIO, file_path: String, content: Vec<u8>) -> anyhow::Result<()> {
  let r2 = R2Address::from_string(file_path)?;
  io.put(&r2, content).await.map_err(to_anyhow)?;
  Ok(())
}*/
