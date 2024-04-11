use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{merge_headers, merge_key_value_vecs};
use crate::config::headers::Headers;
use crate::config::KeyValue;
use crate::is_default;
use crate::merge_right::MergeRight;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
/// The `@server` directive, when applied at the schema level, offers a
/// comprehensive set of server configurations. It dictates how the server
/// behaves and helps tune tailcall for various use-cases.
pub struct Server {
    #[serde(default, skip_serializing_if = "is_default")]
    /// `apolloTracing` exposes GraphQL query performance data, including
    /// execution time of queries and individual resolvers.
    pub apollo_tracing: Option<bool>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `batchRequests` combines multiple requests into one, improving
    /// performance but potentially introducing latency and complicating
    /// debugging. Use judiciously. @default `false`.
    pub batch_requests: Option<bool>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `headers` contains key-value pairs that are included as default headers
    /// in server responses, allowing for consistent header management across
    /// all responses.
    pub headers: Option<Headers>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `globalResponseTimeout` sets the maximum query duration before
    /// termination, acting as a safeguard against long-running queries.
    pub global_response_timeout: Option<i64>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `graphiql` activates the GraphiQL IDE at the root path within Tailcall,
    /// a tool for query development and testing. @default `false`.
    pub graphiql: Option<bool>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `hostname` sets the server hostname.
    pub hostname: Option<String>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `introspection` allows clients to fetch schema information directly,
    /// aiding tools and applications in understanding available types, fields,
    /// and operations. @default `true`.
    pub introspection: Option<bool>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `pipelineFlush` allows to control flushing behavior of the server
    /// pipeline.
    pub pipeline_flush: Option<bool>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `port` sets the Tailcall running port. @default `8000`.
    pub port: Option<u16>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `queryValidation` checks incoming GraphQL queries against the schema,
    /// preventing errors from invalid queries. Can be disabled for performance.
    /// @default `false`.
    pub query_validation: Option<bool>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `responseValidation` Tailcall automatically validates responses from
    /// upstream services using inferred schema. @default `false`.
    pub response_validation: Option<bool>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// A link to an external JS file that listens on every HTTP request
    /// response event.
    pub script: Option<ScriptOptions>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `showcase` enables the /showcase/graphql endpoint.
    pub showcase: Option<bool>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// This configuration defines local variables for server operations. Useful
    /// for storing constant configurations, secrets, or shared information.
    pub vars: Vec<KeyValue>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `version` sets the HTTP version for the server. Options are `HTTP1` and
    /// `HTTP2`. @default `HTTP1`.
    pub version: Option<HttpVersion>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `workers` sets the number of worker threads. @default the number of
    /// system cores.
    pub workers: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScriptOptions {
    pub timeout: Option<u64>,
}

impl MergeRight for ScriptOptions {
    fn merge_right(self, other: Self) -> Self {
        ScriptOptions { timeout: self.timeout.merge_right(other.timeout) }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, Default, schemars::JsonSchema)]
pub enum HttpVersion {
    #[default]
    HTTP1,
    HTTP2,
}

impl MergeRight for HttpVersion {
    fn merge_right(self, other: Self) -> Self {
        other
    }
}

impl Server {
    pub fn enable_apollo_tracing(&self) -> bool {
        self.apollo_tracing.unwrap_or(false)
    }
    pub fn enable_graphiql(&self) -> bool {
        self.graphiql.unwrap_or(false)
    }
    pub fn get_global_response_timeout(&self) -> i64 {
        self.global_response_timeout.unwrap_or(0)
    }

    pub fn get_workers(&self) -> usize {
        self.workers.unwrap_or(num_cpus::get())
    }

    pub fn get_port(&self) -> u16 {
        self.port.unwrap_or(8000)
    }
    pub fn enable_http_validation(&self) -> bool {
        self.response_validation.unwrap_or(false)
    }
    pub fn enable_cache_control(&self) -> bool {
        self.headers
            .as_ref()
            .map(|h| h.enable_cache_control())
            .unwrap_or(false)
    }
    pub fn enable_set_cookies(&self) -> bool {
        self.headers
            .as_ref()
            .map(|h| h.set_cookies())
            .unwrap_or(false)
    }
    pub fn enable_introspection(&self) -> bool {
        self.introspection.unwrap_or(true)
    }
    pub fn enable_query_validation(&self) -> bool {
        self.query_validation.unwrap_or(false)
    }
    pub fn enable_batch_requests(&self) -> bool {
        self.batch_requests.unwrap_or(false)
    }
    pub fn enable_showcase(&self) -> bool {
        self.showcase.unwrap_or(false)
    }

    pub fn get_hostname(&self) -> String {
        self.hostname.clone().unwrap_or("127.0.0.1".to_string())
    }

    pub fn get_vars(&self) -> BTreeMap<String, String> {
        self.vars
            .clone()
            .iter()
            .map(|kv| (kv.key.clone(), kv.value.clone()))
            .collect()
    }

    pub fn get_response_headers(&self) -> Vec<(String, String)> {
        self.headers
            .as_ref()
            .map(|h| h.custom.clone())
            .map_or(Vec::new(), |headers| {
                headers
                    .iter()
                    .map(|kv| (kv.key.clone(), kv.value.clone()))
                    .collect()
            })
    }

    pub fn get_experimental_headers(&self) -> BTreeSet<String> {
        self.headers
            .as_ref()
            .map(|h| h.experimental.clone().unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn get_version(self) -> HttpVersion {
        self.version.unwrap_or(HttpVersion::HTTP1)
    }

    pub fn get_pipeline_flush(&self) -> bool {
        self.pipeline_flush.unwrap_or(true)
    }
}

impl MergeRight for Server {
    fn merge_right(mut self, other: Self) -> Self {
        self.apollo_tracing = self.apollo_tracing.merge_right(other.apollo_tracing);
        self.headers = merge_headers(self.headers, other.headers);
        self.graphiql = self.graphiql.merge_right(other.graphiql);
        self.introspection = self.introspection.merge_right(other.introspection);
        self.query_validation = self.query_validation.merge_right(other.query_validation);
        self.response_validation = self
            .response_validation
            .merge_right(other.response_validation);
        self.batch_requests = self.batch_requests.merge_right(other.batch_requests);
        self.global_response_timeout = self
            .global_response_timeout
            .merge_right(other.global_response_timeout);
        self.showcase = self.showcase.merge_right(other.showcase);
        self.workers = self.workers.merge_right(other.workers);
        self.port = self.port.merge_right(other.port);
        self.hostname = self.hostname.merge_right(other.hostname);
        self.vars = other.vars.iter().fold(self.vars.to_vec(), |mut acc, kv| {
            let position = acc.iter().position(|x| x.key == kv.key);
            if let Some(pos) = position {
                acc[pos] = kv.clone();
            } else {
                acc.push(kv.clone());
            };
            acc
        });
        self.vars = merge_key_value_vecs(&self.vars, &other.vars);
        self.version = self.version.merge_right(other.version);
        self.pipeline_flush = self.pipeline_flush.merge_right(other.pipeline_flush);
        self.script = self.script.merge_right(other.script);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScriptOptions;

    fn server_with_script_options(so: ScriptOptions) -> Server {
        Server { script: Some(so), ..Default::default() }
    }

    #[test]
    fn script_options_merge_both() {
        let a = server_with_script_options(ScriptOptions { timeout: Some(100) });
        let b = server_with_script_options(ScriptOptions { timeout: Some(200) });
        let merged = a.merge_right(b);
        let expected = ScriptOptions { timeout: Some(200) };
        assert_eq!(merged.script, Some(expected));
    }

    #[test]
    fn script_options_merge_first() {
        let a = server_with_script_options(ScriptOptions { timeout: Some(100) });
        let b = server_with_script_options(ScriptOptions { timeout: None });
        let merged = a.merge_right(b);
        let expected = ScriptOptions { timeout: Some(100) };
        assert_eq!(merged.script, Some(expected));
    }

    #[test]
    fn script_options_merge_second() {
        let a = server_with_script_options(ScriptOptions { timeout: None });
        let b = server_with_script_options(ScriptOptions { timeout: Some(100) });
        let merged = a.merge_right(b);
        let expected = ScriptOptions { timeout: Some(100) };
        assert_eq!(merged.script, Some(expected));
    }

    #[test]
    fn script_options_merge_second_default() {
        let a = server_with_script_options(ScriptOptions { timeout: Some(100) });
        let b = Server::default();
        let merged = a.merge_right(b);
        let expected = ScriptOptions { timeout: Some(100) };
        assert_eq!(merged.script, Some(expected));
    }

    #[test]
    fn script_options_merge_first_default() {
        let a = Server::default();
        let b = server_with_script_options(ScriptOptions { timeout: Some(100) });
        let merged = a.merge_right(b);
        let expected = ScriptOptions { timeout: Some(100) };
        assert_eq!(merged.script, Some(expected));
    }
}
