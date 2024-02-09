use std::panic;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::anyhow;
use async_graphql_value::ConstValue;
use tailcall::target_runtime::TargetRuntime;
use tailcall::{EnvIO, FileIO, HttpIO};

mod cache;
mod env;
mod file;
pub mod handle;
mod http;

pub fn init_env(env: Rc<worker::Env>) -> Arc<dyn EnvIO> {
    Arc::new(env::CloudflareEnv::init(env))
}

pub fn init_file(env: Rc<worker::Env>, bucket_id: String) -> anyhow::Result<Arc<dyn FileIO>> {
    Ok(Arc::new(file::CloudflareFileIO::init(env, bucket_id)?))
}

pub fn init_http() -> Arc<dyn HttpIO> {
    Arc::new(http::CloudflareHttp::init())
}

pub fn init_cache(env: Rc<worker::Env>) -> Arc<dyn tailcall::Cache<Key = u64, Value = ConstValue>> {
    Arc::new(cache::CloudflareChronoCache::init(env))
}

pub fn init_runtime(env: Rc<worker::Env>) -> anyhow::Result<TargetRuntime> {
    let http = init_http();
    let env_io = init_env(env.clone());
    let bucket_id = env_io
        .get("BUCKET")
        .ok_or(anyhow!("BUCKET var is not set"))?;

    log::debug!("R2 Bucket ID: {}", bucket_id);

    Ok(TargetRuntime {
        http: http.clone(),
        http2_only: http.clone(),
        env: init_env(env.clone()),
        file: init_file(env.clone(), bucket_id)?,
        cache: init_cache(env),
    })
}

#[worker::event(fetch)]
async fn fetch(
    req: worker::Request,
    env: worker::Env,
    ctx: worker::Context,
) -> anyhow::Result<worker::Response> {
    let result = handle::fetch(req, env, ctx).await;

    match result {
        Ok(response) => Ok(response),
        Err(message) => {
            log::error!("ServerError: {}", message.to_string());
            worker::Response::error(message.to_string(), 500).map_err(to_anyhow)
        }
    }
}

#[worker::event(start)]
fn start() {
    // Initialize Logger
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

fn to_anyhow<T: std::fmt::Display>(e: T) -> anyhow::Error {
    anyhow!("{}", e)
}
