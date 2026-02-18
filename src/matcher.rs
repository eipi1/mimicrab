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
        && !path_matches(cond_path, path)
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

fn path_matches(cond_path: &str, target_path: &str) -> bool {
    // If it's a perfect match, no need for regex
    if cond_path == target_path {
        return true;
    }

    // Convert cond_path to regex
    // 1. Replace :param with ([^/]+)
    // 2. Replace * with .*
    // 3. Wrap with ^ and $

    let mut regex_str = String::from("^");
    let segments: Vec<&str> = cond_path.split('/').collect();

    for (i, segment) in segments.iter().enumerate() {
        if i > 0 {
            regex_str.push('/');
        }
        if segment.starts_with(':') {
            regex_str.push_str("([^/]+)");
        } else if *segment == "*" {
            regex_str.push_str(".*");
        } else if segment.contains('*') {
            // Handle mid-segment wildcard like "books*"
            let part = segment.replace('*', ".*");
            regex_str.push_str(&part);
        } else {
            regex_str.push_str(&regex::escape(segment));
        }
    }
    regex_str.push('$');

    if let Ok(re) = regex::Regex::new(&regex_str) {
        re.is_match(target_path)
    } else {
        tracing::error!("Invalid path regex generated: {}", regex_str);
        false
    }
}
