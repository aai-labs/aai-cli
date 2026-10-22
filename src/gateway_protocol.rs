use reqwest::Method;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;
pub const EXECUTE_PATH: &str = "/api/v1/execute";
pub const RESPONSE_SOURCE_HEADER: &str = "x-aai-response-source";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteMetadata {
    pub protocol_version: u32,
    pub profile_id: String,
    pub service: String,
    pub operation: String,
    pub method: String,
    pub url: String,
    pub content_type: Option<String>,
    pub accept: Option<String>,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
}

impl ExecuteMetadata {
    pub fn method(&self) -> Result<Method, String> {
        self.method
            .parse::<Method>()
            .map_err(|_| format!("unsupported HTTP method {}", self.method))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayErrorBody {
    pub code: String,
    pub message: String,
    pub service: String,
    pub operation: String,
    pub status: Option<u16>,
    pub details: Option<serde_json::Value>,
}
