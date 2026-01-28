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
pub struct MockResponse {
    pub status_code: Option<u16>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<serde_json::Value>,
    pub latency: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Expectation {
    pub id: u64,
    pub condition: RequestCondition,
    pub response: MockResponse,
}
