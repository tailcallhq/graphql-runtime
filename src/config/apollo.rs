use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq, schemars::JsonSchema)]
pub struct Apollo {
    ///
    /// Setting `api_key` for Apollo.
    pub api_key: String,
    ///
    /// Setting `graph_ref` for Apollo in the format <graph_id>@<variant>.
    pub graph_ref: String,
    ///
    /// Setting `userVersion` for Apollo.
    #[serde(default = "default_user_version")]
    pub user_version: String,
    ///
    /// Setting `platform` for Apollo.
    #[serde(default = "default_platform")]
    pub platform: String,
    ///
    /// Setting `version` for Apollo.
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_user_version() -> String {
    "1.0".to_string()
}

fn default_platform() -> String {
    "platform".to_string()
}

fn default_version() -> String {
    "1.0".to_string()
}
