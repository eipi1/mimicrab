use serde_json::Value;
use regex::Regex;
use once_cell::sync::Lazy;

static PATH_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{\{path\[(\d+)\]\}\}").unwrap());
static BODY_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{\{body\.([a-zA-Z0-9\._]+)\}\}").unwrap());

pub fn resolve_template(template: &str, path_segments: &[&str], body: &Option<Value>) -> String {
    let mut resolved = template.to_string();

    // Resolve path segments: {{path[0]}}, {{path[1]}}, etc.
    resolved = PATH_RE.replace_all(&resolved, |caps: &regex::Captures| {
        let index: usize = caps[1].parse().unwrap_or(999);
        path_segments.get(index).cloned().unwrap_or("null")
    }).to_string();

    // Resolve body values: {{body.some.field}}
    resolved = BODY_RE.replace_all(&resolved, |caps: &regex::Captures| {
        let path = &caps[1];
        if let Some(body_val) = body {
            get_value_by_path(body_val, path)
                .map(|v| v.as_str().unwrap_or(&v.to_string()).replace("\"", ""))
                .unwrap_or_else(|| "null".to_string())
        } else {
            "null".to_string()
        }
    }).to_string();

    resolved
}

fn get_value_by_path<'a>(body: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = body;
    for part in path.split('.') {
        if let Some(next) = current.get(part) {
            current = next;
        } else {
            return None;
        }
    }
    Some(current)
}
