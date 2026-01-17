use crate::models::RequestCondition;
use axum::http::{Method, HeaderMap};
use serde_json::Value;

pub fn matches(
    method: &Method,
    path: &str,
    headers: &HeaderMap,
    body: &Option<Value>,
    condition: &RequestCondition,
) -> bool {
    // Match method
    if let Some(ref cond_method) = condition.method {
        if method.as_str().to_uppercase() != cond_method.to_uppercase() {
            return false;
        }
    }

    // Match path
    if let Some(ref cond_path) = condition.path {
        if path != cond_path {
            return false;
        }
    }

    // Match headers
    if let Some(ref cond_headers) = condition.headers {
        for (key, value) in cond_headers {
            if let Some(header_value) = headers.get(key) {
                if header_value.to_str().unwrap_or("") != value {
                    return false;
                }
            } else {
                return false;
            }
        }
    }

    // Match body
    if let Some(ref cond_body) = condition.body {
        if let Some(req_body) = body {
            if req_body != cond_body {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}
