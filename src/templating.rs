use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

static PATH_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\{path\[(\d+)\](?::(string))?\}\}").unwrap());
static BODY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\{body([\.\[][a-zA-Z0-9\._\[\]]+)(?::(string))?\}\}").unwrap());

pub fn resolve_template(template: &str, path_segments: &[&str], body: &Option<Value>) -> String {
    let mut resolved = template.to_string();

    // Resolve path segments: {{path[0]}}, {{path[1]}}, etc.
    resolved = PATH_RE
        .replace_all(&resolved, |caps: &regex::Captures| {
            let index: usize = caps[1].parse().unwrap_or(999);
            let val = path_segments.get(index).cloned().unwrap_or("null");
            tracing::trace!("Template resolved path[{}]: {}", index, val);
            val.to_string()
        })
        .to_string();

    // Resolve body values: {{body.some.field}} or {{body[0].name}}
    resolved = BODY_RE
        .replace_all(&resolved, |caps: &regex::Captures| {
            let path = &caps[1];
            let val = if let Some(body_val) = body {
                get_value_by_path(body_val, path)
                    .map(|v| {
                        if v.is_string() {
                            v.as_str().unwrap().to_string()
                        } else {
                            v.to_string().replace("\"", "")
                        }
                    })
                    .unwrap_or_else(|| "null".to_string())
            } else {
                "null".to_string()
            };
            tracing::trace!("Template resolved body{}: {}", path, val);
            val
        })
        .to_string();

    tracing::trace!("Final resolved template: {}", resolved);
    resolved
}

pub fn resolve_template_value(
    res_body: Value,
    path_segments: &[&str],
    req_body: &Option<Value>,
) -> Value {
    match res_body {
        Value::String(s) => {
            // Check if it's a single template marker
            if let Some(caps) = PATH_RE.captures(&s)
                && caps[0] == s
            {
                let index: usize = caps[1].parse().unwrap_or(999);
                let force_string = caps.get(2).is_some();
                let raw_val = path_segments.get(index).cloned().unwrap_or("null");
                if force_string {
                    return Value::String(raw_val.to_string());
                }
                return attempt_parse_string(raw_val);
            }
            if let Some(caps) = BODY_RE.captures(&s)
                && caps[0] == s
            {
                let path = &caps[1];
                let force_string = caps.get(2).is_some();
                if let Some(body_val) = req_body
                    && let Some(v) = get_value_by_path(body_val, path)
                {
                    if force_string {
                        return Value::String(match v {
                            Value::String(s) => s.clone(),
                            _ => v.to_string(),
                        });
                    }
                    return v.clone();
                }
                return Value::Null;
            }
            // Fallback to string-based partial resolution
            Value::String(resolve_template(&s, path_segments, req_body))
        }
        Value::Array(arr) => Value::Array(
            arr.into_iter()
                .map(|v| resolve_template_value(v, path_segments, req_body))
                .collect(),
        ),
        Value::Object(obj) => Value::Object(
            obj.into_iter()
                .map(|(k, v)| (k, resolve_template_value(v, path_segments, req_body)))
                .collect(),
        ),
        _ => res_body,
    }
}

fn attempt_parse_string(s: &str) -> Value {
    if s == "true" {
        return Value::Bool(true);
    }
    if s == "false" {
        return Value::Bool(false);
    }
    if let Ok(i) = s.parse::<i64>() {
        return Value::Number(i.into());
    }
    if let Ok(f) = s.parse::<f64>()
        && let Some(n) = serde_json::Number::from_f64(f)
    {
        return Value::Number(n);
    }
    Value::String(s.to_string())
}

fn get_value_by_path<'a>(body: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = body;
    // Normalize path: replace [n] with .n
    let normalized = path.replace('[', ".").replace(']', "");
    for part in normalized.split('.') {
        if part.is_empty() {
            continue;
        }
        if let Ok(index) = part.parse::<usize>() {
            current = current.get(index)?;
        } else {
            current = current.get(part)?;
        }
    }
    Some(current)
}
