use std::collections::HashMap;

use chrono::NaiveDateTime;
use reqwest::header::{HeaderName, HeaderValue};
use serde::Serialize;
use serde_json::Value;

use super::super::Result;
use super::Collect;
use crate::Event;

pub struct Tracker {
    api_secret: &'static str,
}

impl Tracker {
    pub fn new(api_secret: &'static str) -> Self {
        Self { api_secret }
    }
}

#[derive(Debug, Serialize)]
struct Payload {
    api_key: String,
    event: String,
    distinct_id: String,
    properties: HashMap<String, serde_json::Value>,
    timestamp: Option<NaiveDateTime>,
}

impl Payload {
    fn new(api_key: String, input: Event) -> Self {
        let mut properties = HashMap::new();
        let distinct_id = input.client_id.to_string();
        let event = input.event_name.to_string();

        if let Ok(Value::Object(map)) = serde_json::to_value(input) {
            for (key, value) in map {
                properties.insert(key, value);
            }
        }

        Self {
            api_key,
            event,
            distinct_id,
            properties,
            timestamp: Some(chrono::Utc::now().naive_utc()),
        }
    }
}

impl Tracker {
    fn create_request(&self, event: Event) -> Result<reqwest::Request> {
        let url = reqwest::Url::parse("https://us.i.posthog.com/capture/")?;
        let mut request = reqwest::Request::new(reqwest::Method::POST, url);
        request.headers_mut().insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        );

        let event = Payload::new(self.api_secret.to_string(), event);

        let _ = request
            .body_mut()
            .insert(reqwest::Body::from(serde_json::to_string(&event)?));

        Ok(request)
    }
}

#[async_trait::async_trait]
impl Collect for Tracker {
    // TODO: move http request to a dispatch
    async fn collect(&self, event: Event) -> Result<()> {
        let request = self.create_request(event)?;
        let client = reqwest::Client::new();
        client.execute(request).await?;

        Ok(())
    }
}
