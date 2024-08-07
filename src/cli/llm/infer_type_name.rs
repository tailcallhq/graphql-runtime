use std::collections::HashMap;

use genai::chat::{ChatMessage, ChatRequest, ChatResponse};
use serde::{Deserialize, Serialize};

use super::{Error, Result, Wizard};
use crate::core::config::Config;

const MODEL: &str = "llama3-8b-8192";

#[derive(Default)]
pub struct InferTypeName {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Answer {
    suggestions: Vec<String>,
}

impl TryFrom<ChatResponse> for Answer {
    type Error = Error;

    fn try_from(response: ChatResponse) -> Result<Self> {
        let json = response.content.ok_or(Error::EmptyResponse)?;
        Ok(serde_json::from_str(json.as_str())?)
    }
}

#[derive(Clone, Serialize)]
struct Question {
    fields: Vec<(String, String)>,
}

impl TryInto<ChatRequest> for Question {
    type Error = Error;

    fn try_into(self) -> Result<ChatRequest> {
        let content = serde_json::to_string(&self)?;
        let input = serde_json::to_string_pretty(&Question {
            fields: vec![
                ("id".to_string(), "String".to_string()),
                ("name".to_string(), "String".to_string()),
                ("age".to_string(), "Int".to_string()),
            ],
        })?;

        let output = serde_json::to_string_pretty(&Answer {
            suggestions: vec![
                "Person".into(),
                "Profile".into(),
                "Member".into(),
                "Individual".into(),
                "Contact".into(),
            ],
        })?;

        Ok(ChatRequest::new(vec![
            ChatMessage::system(
                "Given the sample schema of a GraphQL type suggest 5 meaningful names for it.",
            ),
            ChatMessage::system("The name should be concise and preferably a single word"),
            ChatMessage::system("Example Input:"),
            ChatMessage::system(input),
            ChatMessage::system("Example Output:"),
            ChatMessage::system(output),
            ChatMessage::system("Ensure the output is in valid JSON format".to_string()),
            ChatMessage::system(
                "Do not add any additional text before or after the json".to_string(),
            ),
            ChatMessage::user(content),
        ]))
    }
}

impl InferTypeName {
    pub async fn generate(&mut self, config: &Config) -> Result<HashMap<String, String>> {
        let engine: Wizard<Question, Answer> = Wizard::new(MODEL.to_string());

        let mut new_name_mappings: HashMap<String, String> = HashMap::new();
        let total = config.types.len();
        for (i, (type_name, type_)) in config.types.iter().enumerate() {
            if config.is_root_operation_type(type_name) {
                // Ignore the root types as their names are already given by the user.
                continue;
            }

            // convert type to sdl format.
            let question = Question {
                fields: type_
                    .fields
                    .iter()
                    .map(|(k, v)| (k.clone(), v.type_of.clone()))
                    .collect(),
            };

            let mut delay = 3;
            loop {
                let answer = engine.ask(question.clone()).await;
                match answer {
                    Ok(answer) => {
                        let name = &answer.suggestions.join(", ");
                        for name in answer.suggestions {
                            if config.types.contains_key(&name)
                                || new_name_mappings.contains_key(&name)
                            {
                                continue;
                            }
                            new_name_mappings.insert(name, type_name.to_owned());
                            break;
                        }
                        tracing::info!("Suggestions for {}: [{}] - {}/{}", type_name, name, i, total);
                        break;
                    }
                    Err(e) => {
                        // TODO: log errors after certain number of retries.
                        if let Error::GenAI(_) = e {
                            tracing::info!("Retrying after {} second", delay);
                            tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                            delay *= std::cmp::min(delay * 2, 60);
                        }
                    }
                }
            }
        }

        Ok(new_name_mappings.into_iter().map(|(k, v)| (v, k)).collect())
    }
}
