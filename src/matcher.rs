use crate::models::RequestCondition;
use axum::http::{HeaderMap, Method};
use serde_json::Value;

pub fn matches(
    method: &Method,
    path: &str,
    headers: &HeaderMap,
    body: &Option<Value>,
    condition: &RequestCondition,
) -> bool {
    // Match method
    if let Some(ref cond_method) = condition.method
        && method.as_str().to_uppercase() != cond_method.to_uppercase()
    {
        tracing::trace!("Method mismatch: expected {}, got {}", cond_method, method);
        return false;
    }

    // Match path
    if let Some(ref cond_path) = condition.path
        && path != cond_path
    {
        tracing::trace!("Path mismatch: expected {}, got {}", cond_path, path);
        return false;
    }

    // Match headers
    if let Some(ref cond_headers) = condition.headers {
        for (key, value) in cond_headers {
            if let Some(header_value) = headers.get(key) {
                if header_value.to_str().unwrap_or("") != value {
                    tracing::trace!(
                        "Header mismatch for {}: expected {}, got {:?}",
                        key,
                        value,
                        header_value
                    );
                    return false;
                }
            } else {
                tracing::trace!("Header missing: {}", key);
                return false;
            }
        }
    }

    // Match body
    if let Some(ref cond_body) = condition.body {
        if let Some(req_body) = body {
            if req_body != cond_body {
                tracing::trace!(
                    "Body mismatch: expected {:?}, got {:?}",
                    cond_body,
                    req_body
                );
                return false;
            }
        } else {
            tracing::trace!("Body missing in request, but expected {:?}", cond_body);
            return false;
        }
    }

    tracing::trace!("Match success!");
    true
}
