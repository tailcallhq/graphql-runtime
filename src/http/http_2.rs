#![allow(clippy::too_many_arguments)]
use std::io::BufReader;
use std::sync::Arc;

use anyhow::Result;
use hyper::server::conn::{AddrIncoming, Connecting};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Response, Server};
use hyper_rustls::TlsAcceptor;
use rustls::PrivateKey;
use tokio::fs::File;
use tokio::sync::oneshot;

use super::server_config::ServerConfig;
use super::{handle_request, log_launch_and_open_browser};
use crate::async_graphql_hyper::{GraphQLBatchRequest, GraphQLRequest};
use crate::cli::CLIError;

async fn load_cert(filename: &str) -> Result<Vec<rustls::Certificate>, std::io::Error> {
  let file = File::open(filename).await?;
  let file = file.into_std().await;
  let mut file = BufReader::new(file);

  let certificates = rustls_pemfile::certs(&mut file)?;

  Ok(certificates.into_iter().map(rustls::Certificate).collect())
}

async fn load_private_key(filename: &str) -> anyhow::Result<PrivateKey> {
  let file = File::open(filename).await?;
  let file = file.into_std().await;
  let mut file = BufReader::new(file);

  let keys = rustls_pemfile::read_all(&mut file)?;

  if keys.len() != 1 {
    return Err(CLIError::new("Expected a single private key").into());
  }

  let key = keys.into_iter().find_map(|key| match key {
    rustls_pemfile::Item::RSAKey(key) => Some(PrivateKey(key)),
    rustls_pemfile::Item::ECKey(key) => Some(PrivateKey(key)),
    rustls_pemfile::Item::PKCS8Key(key) => Some(PrivateKey(key)),
    _ => None,
  });

  key.ok_or(CLIError::new("Invalid private key").into())
}

pub async fn start_http_2(
  sc: Arc<ServerConfig>,
  cert: String,
  key: String,
  server_up_sender: Option<oneshot::Sender<()>>,
) -> anyhow::Result<()> {
  let addr = sc.addr();
    let incoming = AddrIncoming::bind(&addr)?;

  if cert.is_empty() | key.is_empty() {
    // let make_svc_single_req = make_service_fn(|_conn| {
    //   let state = Arc::clone(&sc);
    //   async move {
    //     Ok::<_, anyhow::Error>(service_fn(move |req| {
    //       handle_request::<GraphQLRequest>(req, state.server_context.clone())
    //     }))
    //   }
    // });

    // let make_svc_batch_req = make_service_fn(|_conn| {
    //   let state = Arc::clone(&sc);
    //   async move {
    //     Ok::<_, anyhow::Error>(service_fn(move |req| {
    //       handle_request::<GraphQLBatchRequest>(req, state.server_context.clone())
    //     }))
    //   }
    // });

    let builder = Server::builder(incoming);

    let _ = builder
      .serve(make_service_fn(|connection| {
        let state = Arc::clone(&sc);
        log::info!("{:#?}", connection);
        async move {
          Ok::<_, anyhow::Error>(service_fn(|req| async move {
            if req.version() != hyper::Version::HTTP_2 {
              let response = Response::builder()
                .status(101)
                .header("Connection", "Upgrade")
                .header("Upgrade", "h2c")
                .body(Body::default())
                .unwrap();

              // Log information after initiating the upgrade
              println!("Upgrade to HTTP/2 initiated.");

              Ok(response)
            } else {
              Err("Error")
            }
          }))
        }
      }))
      .await;

    log_launch_and_open_browser(sc.as_ref());

    if let Some(sender) = server_up_sender {
      sender.send(()).or(Err(anyhow::anyhow!("Failed to send message")))?;
    }

    // let server: std::prelude::v1::Result<(), hyper::Error> = if sc.blueprint.server.enable_batch_requests {
    //   builder.serve(make_svc_batch_req).await
    // } else {
    //   builder.serve(make_svc_single_req).await
    // };

    // let result = server.map_err(CLIError::from);

    // Ok(result?)
    Ok(())
  } else {
    let cert_chain = load_cert(&cert).await?;
    let key = load_private_key(&key).await?;
    let acceptor = TlsAcceptor::builder()
      .with_single_cert(cert_chain, key)?
      .with_http2_alpn()
      .with_incoming(incoming);
    let make_svc_single_req = make_service_fn(|_conn| {
      let state = Arc::clone(&sc);
      async move {
        Ok::<_, anyhow::Error>(service_fn(move |req| {
          handle_request::<GraphQLRequest>(req, state.server_context.clone())
        }))
      }
    });

    let make_svc_batch_req = make_service_fn(|_conn| {
      let state = Arc::clone(&sc);
      async move {
        Ok::<_, anyhow::Error>(service_fn(move |req| {
          handle_request::<GraphQLBatchRequest>(req, state.server_context.clone())
        }))
      }
    });

    let builder = Server::builder(acceptor).http2_only(true);

    log_launch_and_open_browser(sc.as_ref());

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
}
