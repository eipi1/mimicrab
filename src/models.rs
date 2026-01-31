use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestCondition {
    pub method: Option<String>,
    pub path: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseConfig {
    pub status_code: Option<u16>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<serde_json::Value>,
    pub body_type: Option<String>,
    pub latency: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JitterConfig {
    pub probability: f64,
    #[serde(flatten)]
    pub response: ResponseConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyConfig {
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MockResponse {
    #[serde(flatten)]
    pub response: ResponseConfig,
    pub jitter: Option<JitterConfig>,
    pub proxy: Option<ProxyConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Expectation {
    pub id: u64,
    pub condition: RequestCondition,
    pub response: MockResponse,
}
