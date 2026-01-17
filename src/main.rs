mod models;
mod matcher;
mod templating;

use axum::{
    extract::{Path as AxPath, Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response, sse::{Event, Sse}},
    routing::{any, get, post, put, delete},
    Router, Json,
};
use axum::body::Body;
use http_body_util::BodyExt;
use models::Expectation;
use std::{fs, sync::Arc, convert::Infallible};
use tokio::sync::{broadcast, RwLock};
use serde_json::{json, Value};
use futures::stream::Stream;

#[derive(Clone, Debug, serde::Serialize)]
struct LogEntry {
    timestamp: String,
    method: String,
    path: String,
    body: Option<Value>,
    matched: bool,
    expectation_id: Option<u64>,
}

struct AppState {
    expectations: RwLock<Vec<Expectation>>,
    log_tx: broadcast::Sender<LogEntry>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    let expectations_path = "expectations.json";
    let initial_expectations = load_expectations(expectations_path);
    let (log_tx, _) = broadcast::channel(100);
    
    let state = Arc::new(AppState {
        expectations: RwLock::new(initial_expectations),
        log_tx,
    });

    let admin_router = Router::new()
        .route("/mocks", get(list_mocks).post(add_mock))
        .route("/mocks/:id", put(update_mock).delete(delete_mock))
        .route("/logs/stream", get(stream_logs));

    let app = Router::new()
        .nest("/_admin", admin_router)
        .fallback(any(handle_request))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Mock server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

fn load_expectations(path: &str) -> Vec<Expectation> {
    if std::path::Path::new(path).exists() {
        let content = fs::read_to_string(path).expect("Failed to read expectations file");
        serde_json::from_str(&content).expect("Failed to parse expectations JSON")
    } else {
        vec![]
    }
}

// Admin Handlers
async fn list_mocks(State(state): State<Arc<AppState>>) -> Json<Vec<Expectation>> {
    Json(state.expectations.read().await.clone())
}

async fn add_mock(
    State(state): State<Arc<AppState>>,
    Json(new_mock): Json<Expectation>,
) -> (StatusCode, Json<Expectation>) {
    let mut mocks = state.expectations.write().await;
    mocks.push(new_mock.clone());
    (StatusCode::CREATED, Json(new_mock))
}

async fn update_mock(
    State(state): State<Arc<AppState>>,
    AxPath(id): AxPath<u64>,
    Json(updated_mock): Json<Expectation>,
) -> StatusCode {
    let mut mocks = state.expectations.write().await;
    if let Some(pos) = mocks.iter().position(|m| m.id == id) {
        mocks[pos] = updated_mock;
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn delete_mock(
    State(state): State<Arc<AppState>>,
    AxPath(id): AxPath<u64>,
) -> StatusCode {
    let mut mocks = state.expectations.write().await;
    if let Some(pos) = mocks.iter().position(|m| m.id == id) {
        mocks.remove(pos);
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn stream_logs(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.log_tx.subscribe();
    
    let stream = async_stream::stream! {
        while let Ok(msg) = rx.recv().await {
            yield Ok(Event::default().json_data(msg).unwrap());
        }
    };
    
    Sse::new(stream)
}

async fn handle_request(
    State(state): State<Arc<AppState>>,
    req: Request,
) -> Response {
    let (parts, body) = req.into_parts();
    let path = parts.uri.path();
    let method = &parts.method;
    let headers = &parts.headers;

    // Read body
    let body_bytes = body.collect().await.map(|b| b.to_bytes()).unwrap_or_default();
    let body_json: Option<Value> = serde_json::from_slice(&body_bytes).ok();

    tracing::info!("Incoming request: {} {}", method, path);

    let expectations = state.expectations.read().await;
    let matched = expectations.iter().find(|exp| {
        matcher::matches(method, path, headers, &body_json, &exp.condition)
    });

    let log_entry = LogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        method: method.to_string(),
        path: path.to_string(),
        body: body_json.clone(),
        matched: matched.is_some(),
        expectation_id: matched.map(|e| e.id),
    };
    let _ = state.log_tx.send(log_entry);

    if let Some(exp) = matched {
        tracing::info!("Matched expectation: {}", exp.id);
        
        let status = StatusCode::from_u16(exp.response.status_code.unwrap_or(200)).unwrap_or(StatusCode::OK);
        let mut response_builder = Response::builder().status(status);

        if let Some(ref res_headers) = exp.response.headers {
            for (key, value) in res_headers {
                response_builder = response_builder.header(key, value);
            }
        }

        // Handle Parameterization
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        let response_body = if let Some(ref res_body) = exp.response.body {
            let body_str = serde_json::to_string(res_body).unwrap();
            let resolved_body = templating::resolve_template(&body_str, &path_segments, &body_json);
            
            response_builder = response_builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(resolved_body)
        } else {
            Body::empty()
        };

        response_builder.body(response_body).unwrap()
    } else {
        tracing::warn!("No match found for {} {}", method, path);
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "No matching response found",
                "request": {
                    "method": method.as_str(),
                    "path": path,
                    "body": body_json
                }
            })),
        ).into_response()
    }
}
