use std::time::Duration;

use http_cache_reqwest::{Cache, CacheMode, HttpCache, HttpCacheOptions, MokaManager};
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};

use super::Response;
use crate::config::{self, Upstream};

#[async_trait::async_trait]
pub trait HttpClient: Sync + Send {
  async fn execute(&self, req: reqwest::Request) -> anyhow::Result<Response>;
}

#[async_trait::async_trait]
impl HttpClient for DefaultHttpClient {
  async fn execute(&self, req: reqwest::Request) -> anyhow::Result<Response> {
    let response = self.execute(req).await?;
    Ok(response)
  }
}

#[derive(Clone)]
pub struct DefaultHttpClient {
  client: ClientWithMiddleware,
}

impl Default for DefaultHttpClient {
  fn default() -> Self {
    let upstream = config::Upstream::default();
    //TODO: default is used only in tests. Drop default and move it to test.
    DefaultHttpClient::new(&upstream)
  }
}

impl DefaultHttpClient {
  pub fn new(upstream: &Upstream) -> Self {
    let mut builder = Client::builder()
      .tcp_keepalive(Some(Duration::from_secs(upstream.get_tcp_keep_alive())))
      .timeout(Duration::from_secs(upstream.get_timeout()))
      .connect_timeout(Duration::from_secs(upstream.get_connect_timeout()))
      .http2_keep_alive_interval(Some(Duration::from_secs(upstream.get_keep_alive_interval())))
      .http2_keep_alive_timeout(Duration::from_secs(upstream.get_keep_alive_timeout()))
      .http2_keep_alive_while_idle(upstream.get_keep_alive_while_idle())
      .pool_idle_timeout(Some(Duration::from_secs(upstream.get_pool_idle_timeout())))
      .pool_max_idle_per_host(upstream.get_pool_max_idle_per_host())
      .user_agent(upstream.get_user_agent());

    if let Some(ref proxy) = upstream.proxy {
      builder = builder.proxy(reqwest::Proxy::http(proxy.url.clone()).expect("Failed to set proxy in http client"));
    }

    let mut client = ClientBuilder::new(builder.build().expect("Failed to build client"));

    if upstream.get_enable_http_cache() {
      client = client.with(Cache(HttpCache {
        mode: CacheMode::Default,
        manager: MokaManager::default(),
        options: HttpCacheOptions::default(),
      }))
    }

    DefaultHttpClient { client: client.build() }
  }

  pub async fn execute(&self, request: reqwest::Request) -> reqwest_middleware::Result<Response> {
    log::info!("{} {} ", request.method(), request.url());
    let response = self.client.execute(request).await?;
    let response = Response::from_response(response).await?;
    Ok(response)
  }
}
