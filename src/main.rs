mod kubernetes;
mod matcher;
mod models;
mod templating;

use arc_swap::ArcSwap;
use axum::body::Body;
use axum::{
    Json, Router,
    extract::{Path as AxPath, Request, State},
    http::{StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
    routing::{get, post, put},
};
use futures::stream::Stream;
use http_body_util::BodyExt;
use kube::Client;
use models::Expectation;
use rust_embed::RustEmbed;
use serde_json::{Value, json};
use std::{convert::Infallible, fs, sync::Arc};
use tokio::sync::broadcast;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
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
    expectations: Arc<ArcSwap<Vec<Expectation>>>,
    log_tx: broadcast::Sender<LogEntry>,
    kube_client: Option<Client>,
    config_map_name: String,
    namespace: String,
}

#[derive(RustEmbed)]
#[folder = "ui/"]
struct Assets;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    let expectations_path = "expectations.json";
    let (log_tx, _) = broadcast::channel(100);

    // Environment Detection
    let is_k8s = std::env::var("KUBERNETES_SERVICE_HOST").is_ok();
    let kube_client = if is_k8s {
        tracing::info!("Mimicrab starting in KUBERNETES mode");
        Client::try_default().await.ok()
    } else {
        tracing::info!("Mimicrab starting in LOCAL mode");
        None
    };

    let config_map_name =
        std::env::var("CONFIG_MAP_NAME").unwrap_or_else(|_| "mimicrab-config".to_string());
    let namespace = std::env::var("KUBERNETES_NAMESPACE").unwrap_or_else(|_| "default".to_string());

    let initial_expectations = if let Some(ref client) = kube_client {
        kubernetes::load_from_configmap(client, &config_map_name, &namespace)
            .await
            .unwrap_or_else(|_| load_expectations(expectations_path))
    } else {
        load_expectations(expectations_path)
    };

    let expectations = Arc::new(ArcSwap::from_pointee(initial_expectations));

    let state = Arc::new(AppState {
        expectations: Arc::clone(&expectations),
        log_tx,
        kube_client,
        config_map_name,
        namespace,
    });

    if let Some(ref client) = state.kube_client {
        let expectations_clone = Arc::clone(&expectations);
        tokio::spawn(kubernetes::run_configmap_watcher(
            client.clone(),
            state.namespace.clone(),
            state.config_map_name.clone(),
            expectations_clone,
        ));
    }

    let admin_router = Router::new()
        .route("/mocks", get(list_mocks).post(add_mock))
        .route("/mocks/{id}", put(update_mock).delete(delete_mock))
        .route("/logs/stream", get(stream_logs))
        .route("/export", get(export_mocks))
        .route("/import", post(import_mocks));

    let app = Router::new()
        .nest("/_admin", admin_router)
        .route("/ui/{*path}", get(static_handler))
        .route("/ui/", get(static_handler))
        .route(
            "/",
            get(|| async { axum::response::Redirect::permanent("/ui/") }),
        )
        .fallback(handle_request)
        .layer(TraceLayer::new_for_http())
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

fn save_expectations(path: &str, expectations: &[Expectation]) {
    let content =
        serde_json::to_string_pretty(expectations).expect("Failed to serialize expectations");
    fs::write(path, content).expect("Failed to write expectations file");
    tracing::info!("Local expectations saved to {}", path);
}

// Helper struct for adding/cloning mocks
#[derive(Debug, serde::Deserialize)]
struct MockRequest {
    id: Option<u64>,
    condition: models::RequestCondition,
    response: models::MockResponse,
}

// Admin Handlers
async fn list_mocks(State(state): State<Arc<AppState>>) -> Json<Vec<Expectation>> {
    Json((**state.expectations.load()).clone())
}

async fn add_mock(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MockRequest>,
) -> (StatusCode, Json<Expectation>) {
    let mut mocks = (*state.expectations.load_full()).clone();

    let id = req
        .id
        .unwrap_or_else(|| mocks.iter().map(|m| m.id).max().unwrap_or(0) + 1);

    let new_mock = Expectation {
        id,
        condition: req.condition,
        response: req.response,
    };
    mocks.push(new_mock.clone());
    state.expectations.store(Arc::new(mocks.clone()));

    if let Some(ref client) = state.kube_client {
        kubernetes::sync_to_configmap(client, &state.namespace, &state.config_map_name, &mocks)
            .await;
    } else {
        save_expectations("expectations.json", &mocks);
    }

    (StatusCode::CREATED, Json(new_mock))
}

async fn update_mock(
    State(state): State<Arc<AppState>>,
    AxPath(id): AxPath<u64>,
    Json(updated_mock): Json<Expectation>,
) -> StatusCode {
    let mut mocks = (*state.expectations.load_full()).clone();
    if let Some(pos) = mocks.iter().position(|m| m.id == id) {
        mocks[pos] = updated_mock;
        state.expectations.store(Arc::new(mocks.clone()));

        if let Some(ref client) = state.kube_client {
            kubernetes::sync_to_configmap(client, &state.namespace, &state.config_map_name, &mocks)
                .await;
        } else {
            save_expectations("expectations.json", &mocks);
        }
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn delete_mock(State(state): State<Arc<AppState>>, AxPath(id): AxPath<u64>) -> StatusCode {
    let mut mocks = (*state.expectations.load_full()).clone();
    if let Some(pos) = mocks.iter().position(|m| m.id == id) {
        mocks.remove(pos);
        state.expectations.store(Arc::new(mocks.clone()));

        if let Some(ref client) = state.kube_client {
            kubernetes::sync_to_configmap(client, &state.namespace, &state.config_map_name, &mocks)
                .await;
        } else {
            save_expectations("expectations.json", &mocks);
        }
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn export_mocks(State(state): State<Arc<AppState>>) -> Json<Vec<Expectation>> {
    Json((**state.expectations.load()).clone())
}

async fn import_mocks(
    State(state): State<Arc<AppState>>,
    Json(new_mocks): Json<Vec<Expectation>>,
) -> StatusCode {
    state.expectations.store(Arc::new(new_mocks.clone()));

    if let Some(ref client) = state.kube_client {
        kubernetes::sync_to_configmap(client, &state.namespace, &state.config_map_name, &new_mocks)
            .await;
    } else {
        save_expectations("expectations.json", &new_mocks);
    }
    StatusCode::OK
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

async fn static_handler(path: Option<AxPath<String>>) -> impl IntoResponse {
    let path = path
        .map(|AxPath(p)| p)
        .unwrap_or_else(|| "index.html".to_string());
    let path = if path.is_empty() || path == "/" {
        "index.html"
    } else {
        &path
    };

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data))
                .unwrap()
        }
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

async fn handle_request(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let (parts, body) = req.into_parts();
    let path = parts.uri.path();
    let method = &parts.method;
    let headers = &parts.headers;

    // Read body
    let body_bytes = body
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let body_json: Option<Value> = serde_json::from_slice(&body_bytes).ok();

    tracing::info!("Incoming request: {} {}", method, path);
    tracing::trace!("Request headers: {:?}", headers);
    tracing::trace!("Parsed request body: {:?}", body_json);

    let expectations = state.expectations.load();
    let matched = expectations
        .iter()
        .find(|exp| matcher::matches(method, path, headers, &body_json, &exp.condition));

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

        let status =
            StatusCode::from_u16(exp.response.status_code.unwrap_or(200)).unwrap_or(StatusCode::OK);
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

            let accept_bson = headers
                .get(header::ACCEPT)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.contains("application/bson"))
                .unwrap_or(false);

            tracing::trace!("Accept BSON: {}", accept_bson);

            if accept_bson {
                let val: Value = serde_json::from_str(&resolved_body).unwrap_or(Value::Null);
                tracing::trace!("Converting resolved body to BSON: {:?}", val);
                if let Ok(bson_val) = bson::to_bson(&val) {
                    let mut bytes = Vec::new();
                    if let Some(doc) = bson_val.as_document() {
                        doc.to_writer(&mut bytes).unwrap();
                        response_builder =
                            response_builder.header(header::CONTENT_TYPE, "application/bson");
                        Body::from(bytes)
                    } else if let bson::Bson::Array(arr) = bson_val {
                        // For arrays, we wrap them in a document with a "data" key as BSON must be a document
                        let doc = bson::doc! { "data": arr };
                        doc.to_writer(&mut bytes).unwrap();
                        response_builder =
                            response_builder.header(header::CONTENT_TYPE, "application/bson");
                        Body::from(bytes)
                    } else {
                        // Fallback to JSON for non-object/non-array types or if BSON conversion is simple
                        response_builder =
                            response_builder.header(header::CONTENT_TYPE, "application/json");
                        Body::from(resolved_body)
                    }
                } else {
                    response_builder =
                        response_builder.header(header::CONTENT_TYPE, "application/json");
                    Body::from(resolved_body)
                }
            } else {
                response_builder =
                    response_builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(resolved_body)
            }
        } else {
            Body::empty()
        };

        let response = response_builder.body(response_body).unwrap();
        tracing::info!("Returning matched response: status={}", response.status());
        response
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
        )
            .into_response()
    }
}
