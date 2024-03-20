use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::config::cors::Cors;
use crate::config::KeyValue;
use crate::is_default;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Headers {
    #[serde(default, skip_serializing_if = "is_default")]
    /// `cacheControl` sends `Cache-Control` headers in responses when
    /// activated. The `max-age` value is the least of the values received from
    /// upstream services. @default `false`.
    pub cache_control: Option<bool>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `headers` are key-value pairs included in every server
    /// response. Useful for setting headers like `Access-Control-Allow-Origin`
    /// for cross-origin requests or additional headers for downstream services.
    pub custom: Vec<KeyValue>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `experimental` allows the use of `X-*` experimental headers
    /// in the response. @default `[]`.
    pub experimental: Option<BTreeSet<String>>,

    /// `setCookies` when enabled stores `set-cookie` headers
    /// and all the response will be sent with the headers.
    #[serde(default, skip_serializing_if = "is_default")]
    pub set_cookies: Option<bool>,

    #[serde(default, skip_serializing_if = "is_default")]
    /// `cors` allows Cross-Origin Resource Sharing (CORS) for a server.
    pub cors: Option<Cors>,
}

impl Headers {
    pub fn enable_cache_control(&self) -> bool {
        self.cache_control.unwrap_or(false)
    }
    pub fn set_cookies(&self) -> bool {
        self.set_cookies.unwrap_or_default()
    }
    pub fn get_cors(&self) -> Option<Cors> {
        self.cors.clone()
    }
}

pub fn merge_headers(current: Option<Headers>, other: Option<Headers>) -> Option<Headers> {
    let mut headers = current.clone();

    if let Some(other_headers) = other {
        if let Some(mut self_headers) = current.clone() {
            self_headers.cache_control = other_headers.cache_control.or(self_headers.cache_control);
            self_headers.custom.extend(other_headers.custom);
            self_headers.cors = other_headers.cors.or(self_headers.cors);

            headers = Some(self_headers);
        } else {
            headers = Some(other_headers);
        }
    }

    headers
}
