use derive_setters::Setters;
use genai::chat::{ChatOptions, ChatRequest, ChatResponse};
use genai::Client;

use super::Result;
use crate::cli::llm::adapter::Adapter;

#[derive(Setters, Clone)]
pub struct Wizard<Q, A> {
    client: Client,
    model: Adapter,
    _q: std::marker::PhantomData<Q>,
    _a: std::marker::PhantomData<A>,
}

impl<Q, A> Wizard<Q, A> {
    pub fn new(model: Adapter) -> Self {
        Self {
            client: Client::builder()
                .with_chat_options(
                    ChatOptions::default()
                        .with_json_mode(true)
                        .with_temperature(0.0),
                )
                .build(),
            model,
            _q: Default::default(),
            _a: Default::default(),
        }
    }

    pub async fn ask(&self, q: Q) -> Result<A>
    where
        Q: TryInto<ChatRequest, Error = super::Error>,
        A: TryFrom<ChatResponse, Error = super::Error>,
    {
        let response = self
            .client
            .exec_chat(self.model.to_string().as_str(), q.try_into()?, None)
            .await?;
        A::try_from(response)
    }
}
