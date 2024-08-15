use std::ops::Deref;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::broadcast;
use tokio::sync::oneshot::{self};

use super::http_1::start_http_1;
use super::http_2::start_http_2;
use super::server_config::ServerConfig;
use crate::cli::telemetry::init_opentelemetry;
use crate::cli::CLIError;
use crate::core::blueprint::{Blueprint, Http};
use crate::core::config::ConfigModule;

pub struct Server {
    config_module: ConfigModule,
    server_up_sender: Option<oneshot::Sender<()>>,
}

impl Server {
    pub fn new(config_module: ConfigModule) -> Self {
        Self { config_module, server_up_sender: None }
    }

    pub fn server_up_receiver(&mut self) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();

        self.server_up_sender = Some(tx);

        rx
    }

    /// Starts the server in the current Runtime
    pub async fn start(self) -> Result<()> {
        let blueprint = Blueprint::try_from(&self.config_module).map_err(CLIError::from)?;
        let endpoints = self.config_module.extensions().endpoint_set.clone();
        let server_config = Arc::new(ServerConfig::new(blueprint.clone(), endpoints).await?);

        init_opentelemetry(blueprint.telemetry.clone(), &server_config.app_ctx.runtime)?;

        match blueprint.server.http.clone() {
            Http::HTTP2 { cert, key } => {
                start_http_2(server_config, cert, key, self.server_up_sender).await
            }
            Http::HTTP1 => start_http_1(server_config, self.server_up_sender).await,
        }
    }

    pub async fn fork_start(self, rec: Option<&mut broadcast::Receiver<()>>) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(self.config_module.deref().server.get_workers())
            .enable_all()
            .build()?;

        let handle = runtime.spawn(async { self.start().await });

        if let Some(receiver) = rec {
            tokio::select! {
                _ = receiver.recv() => {
                    tracing::info!("Server shutdown signal received");
                    runtime.shutdown_background();
                    tracing::info!("Server shutdown complete");
                }
                _ = handle => {
                    tracing::info!("Server completed without shutdown signal");
                }
            }
        } else {
            handle.await?;
        }
        Ok(())
    }
}
