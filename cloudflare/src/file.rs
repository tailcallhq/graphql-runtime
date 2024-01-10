use anyhow::Result;
use tailcall::io::FileIO;
pub struct CloudflareFileIO {}

impl CloudflareFileIO {
  pub fn init() -> Self {
    CloudflareFileIO {}
  }
}

// TODO: Temporary implementation that performs an HTTP request to get the file content
// This should be moved to a more native implementation that's based on the WASM env.
#[async_trait::async_trait]
impl FileIO for CloudflareFileIO {
  async fn write<'a>(&'a self, _: &'a str, _: &'a [u8]) -> Result<()> {
    unimplemented!("file write I/O is not required for cloudflare")
  }

  async fn read<'a>(&'a self, _: &'a str) -> Result<(String, String)> {
    unimplemented!("file read I/O is not required for cloudflare")
  }

  async fn read_all<'a>(&'a self, _: &'a [String]) -> Result<Vec<(String, String)>> {
    unimplemented!("file read I/O is not required for cloudflare")
  }
}
