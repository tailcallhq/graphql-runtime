use std::sync::Arc;
use std::time::{Duration, Instant};

use hyper::service::{make_service_fn, service_fn};
use tokio::sync::oneshot;

use super::server_config::ServerConfig;
use super::{handle_request, log_launch};
use crate::async_graphql_hyper::{GraphQLBatchRequest, GraphQLRequest};
use crate::cli::CLIError;

pub async fn start_http_1(
  sc: Arc<ServerConfig>,
  server_up_sender: Option<oneshot::Sender<()>>,
  ttl: Option<u64>,
) -> anyhow::Result<()> {
  let addr = sc.addr();
  let ttl = ttl.unwrap_or_default();
  let mut inst = Instant::now() + Duration::from_secs(ttl);

  let make_svc_single_req = make_service_fn(|_conn| {
    let state = Arc::clone(&sc);
    async move {
      Ok::<_, anyhow::Error>(service_fn(move |req| {
        let now = Instant::now();
        let update_schema = ttl > 0 && (now - inst).as_secs() > ttl;
        if update_schema {
          inst = now;
        }
        handle_request::<GraphQLRequest>(req, state.server_context.clone(), update_schema)
      }))
    }
  });

  let make_svc_batch_req = make_service_fn(|_conn| {
    let state = Arc::clone(&sc);
    async move {
      Ok::<_, anyhow::Error>(service_fn(move |req| {
        let now = Instant::now();
        let update_schema = ttl > 0 && (now - inst).as_secs() > ttl;
        if update_schema {
          inst = now;
        }
        handle_request::<GraphQLBatchRequest>(req, state.server_context.clone(), update_schema)
      }))
    }
  });
  let builder = hyper::Server::try_bind(&addr)
    .map_err(CLIError::from)?
    .http1_pipeline_flush(sc.server_context.blueprint.server.pipeline_flush);
  log_launch(sc.as_ref());

  if let Some(sender) = server_up_sender {
    sender.send(()).or(Err(anyhow::anyhow!("Failed to send message")))?;
  }

  let server: std::prelude::v1::Result<(), hyper::Error> = if sc.blueprint.server.enable_batch_requests {
    builder.serve(make_svc_batch_req).await
  } else {
    builder.serve(make_svc_single_req).await
  };

  let result = server.map_err(CLIError::from);

  Ok(result?)
}
