use std::sync::{Arc, RwLock};

use lazy_static::lazy_static;
use tailcall::async_graphql_hyper::GraphQLRequest;
use tailcall::blueprint::Blueprint;
use tailcall::config::reader::ConfigReader;
use tailcall::config::Config;
use tailcall::http::{handle_request, ServerContext};
use worker::*;

lazy_static! {
  static ref SERV_CTX: RwLock<Option<Arc<ServerContext>>> = RwLock::new(None);
}

async fn make_req() -> Result<Config> {
  let reader = ConfigReader::init(
    [
      "https://raw.githubusercontent.com/tailcallhq/tailcall/main/examples/jsonplaceholder.graphql", // add/edit the SDL links to this list
    ]
    .iter(),
  );
  reader.read().await.map_err(conv_err)
}

#[event(fetch)]
async fn main(req: Request, _: Env, _: Context) -> Result<Response> {
  let mut server_ctx = get_option().await;
  if server_ctx.is_none() {
    let cfg = make_req().await.map_err(conv_err)?;
    let blueprint = Blueprint::try_from(&cfg).map_err(conv_err)?;
    let serv_ctx = Arc::new(ServerContext::new(blueprint));
    *SERV_CTX.write().unwrap() = Some(serv_ctx.clone());
    server_ctx = Some(serv_ctx);
  }
  let resp = handle_request::<GraphQLRequest>(
    convert_to_hyper_request(req).await?,
    server_ctx.ok_or(Error::from("Unable to initiate connection"))?.clone(),
  )
  .await
  .map_err(conv_err)?;
  let resp = make_request(resp).await.map_err(conv_err)?;
  Ok(resp)
}

async fn get_option() -> Option<Arc<ServerContext>> {
  SERV_CTX.read().unwrap().clone()
}

async fn make_request(response: hyper::Response<hyper::Body>) -> Result<Response> {
  let buf = hyper::body::to_bytes(response).await.map_err(conv_err)?;
  let text = std::str::from_utf8(&buf).map_err(conv_err)?;
  let mut response = Response::ok(text).map_err(conv_err)?;
  response
    .headers_mut()
    .append("Content-Type", "text/html")
    .map_err(conv_err)?;
  Ok(response)
}

fn convert_method(worker_method: Method) -> hyper::Method {
  let method_str = &*worker_method.to_string().to_uppercase();

  match method_str {
    "GET" => Ok(hyper::Method::GET),
    "POST" => Ok(hyper::Method::POST),
    "PUT" => Ok(hyper::Method::PUT),
    "DELETE" => Ok(hyper::Method::DELETE),
    "HEAD" => Ok(hyper::Method::HEAD),
    "OPTIONS" => Ok(hyper::Method::OPTIONS),
    "PATCH" => Ok(hyper::Method::PATCH),
    "CONNECT" => Ok(hyper::Method::CONNECT),
    "TRACE" => Ok(hyper::Method::TRACE),
    _ => Err("Unsupported HTTP method"),
  }
  .unwrap()
}

async fn convert_to_hyper_request(mut worker_request: Request) -> Result<hyper::Request<hyper::Body>> {
  let body = worker_request.text().await?;
  let method = worker_request.method();
  let uri = worker_request.url()?.as_str().to_string();
  let headers = worker_request.headers();
  let mut builder = hyper::Request::builder().method(convert_method(method)).uri(uri);
  for (k, v) in headers {
    builder = builder.header(k, v);
  }
  builder.body(hyper::body::Body::from(body)).map_err(conv_err)
}

fn conv_err<T: std::fmt::Display>(e: T) -> Error {
  Error::from(format!("{}", e.to_string()))
}
