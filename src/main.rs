mod kubernetes;
mod matcher;
mod metrics;
mod models;
mod templating;

use arc_swap::ArcSwap;
use axum::body::Body;
use axum::body::Bytes;
use axum::http;
use axum::{
    Json, Router,
    extract::{Path as AxPath, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
    routing::{get, post, put},
};
use clap::Parser;
use futures::stream::Stream;
use http_body_util::BodyExt;
use kube::{Client, Config};
use mlua::{Lua, LuaSerdeExt, Table, Value as LuaValue};
use models::Expectation;
use rust_embed_for_web::{EmbedableFile, RustEmbed};
use serde_json::{Value, json};
use std::{convert::Infallible, fs, sync::Arc};
use tokio::sync::broadcast;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    #[arg(short, long, default_value = "expectations.json")]
    expectations: String,
}
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
    proxy_client: reqwest::Client,
    expectations_path: String,
}

#[derive(RustEmbed)]
#[gzip = true]
#[br = true]
#[folder = "ui/"]
struct Assets;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    metrics::register_process_metrics();

    let args = Args::parse();
    let port = args.port;
    let expectations_path = args.expectations.clone();
    let (log_tx, _) = broadcast::channel(100);

    let provider = rustls::crypto::ring::default_provider();
    rustls::crypto::CryptoProvider::install_default(provider)
        .expect("failed to install crypto provider");

    // Environment Detection
    let is_k8s = std::env::var("KUBERNETES_SERVICE_HOST").is_ok();
    let kube_client = if is_k8s {
        tracing::info!("Mimicrab starting in KUBERNETES mode");
        if let Ok(config) = Config::infer().await {
            tracing::info!("Using Kubernetes config: {:?}", config);
        }
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
            .unwrap_or_else(|_| load_expectations(&expectations_path))
    } else {
        load_expectations(&expectations_path)
    };

    let expectations = Arc::new(ArcSwap::from_pointee(initial_expectations));

    let proxy_client = reqwest::Client::builder()
        .user_agent("mimicrab/0.1.0")
        .build()
        .unwrap();

    let state = Arc::new(AppState {
        expectations: Arc::clone(&expectations),
        log_tx,
        kube_client,
        config_map_name,
        namespace,
        proxy_client,
        expectations_path,
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
        .route("/import", post(import_mocks))
        .route("/metrics", get(metrics_handler));

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

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Mock server running on http://{}", addr);
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
        save_expectations(&state.expectations_path, &mocks);
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
            save_expectations(&state.expectations_path, &mocks);
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
            save_expectations(&state.expectations_path, &mocks);
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
        save_expectations(&state.expectations_path, &new_mocks);
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

async fn static_handler(path: Option<AxPath<String>>, headers: HeaderMap) -> Response {
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
            // Cache validation
            if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH)
                && *if_none_match == *content.etag()
            {
                return StatusCode::NOT_MODIFIED.into_response();
            }

            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut builder = Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .header(header::ETAG, content.etag());

            if let Some(last_modified) = content.last_modified() {
                builder = builder.header(header::LAST_MODIFIED, last_modified);
            }

            // Compression negotiation
            let accept_enc = headers
                .get_all(header::ACCEPT_ENCODING)
                .iter()
                .filter_map(|h| h.to_str().ok())
                .collect::<Vec<_>>()
                .join(", ");

            tracing::debug!("Request Accept-Encoding: {}", accept_enc);
            tracing::debug!(
                "Asset {} supports: br={}, gzip={}",
                path,
                content.data_br().is_some(),
                content.data_gzip().is_some()
            );

            if accept_enc.contains("br") && content.data_br().is_some() {
                builder
                    .header(header::CONTENT_ENCODING, "br")
                    .body(Body::from(content.data_br().unwrap()))
                    .unwrap()
            } else if accept_enc.contains("gzip") && content.data_gzip().is_some() {
                builder
                    .header(header::CONTENT_ENCODING, "gzip")
                    .body(Body::from(content.data_gzip().unwrap()))
                    .unwrap()
            } else {
                builder.body(Body::from(content.data())).unwrap()
            }
        }
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

async fn execute_lua_script(
    script: &str,
    method: &str,
    path: &str,
    headers: &HeaderMap,
    body: &Option<Value>,
) -> Result<Response, String> {
    let lua = Lua::new();

    // Prepare request table
    let req_table = lua.create_table().map_err(|e| e.to_string())?;
    req_table.set("method", method).map_err(|e| e.to_string())?;
    req_table.set("path", path).map_err(|e| e.to_string())?;

    let headers_table = lua.create_table().map_err(|e| e.to_string())?;
    for (name, value) in headers.iter() {
        headers_table
            .set(name.as_str(), value.to_str().unwrap())
            .map_err(|e| e.to_string())?;
    }
    req_table
        .set("headers", headers_table)
        .map_err(|e| e.to_string())?;

    if let Some(body_val) = body {
        let body_lua = lua.to_value(body_val).map_err(|e| e.to_string())?;
        req_table.set("body", body_lua).map_err(|e| e.to_string())?;
    }

    dbg!(&req_table);

    lua.globals()
        .set("request", req_table)
        .map_err(|e| e.to_string())?;

    // Execute script
    let chunk = lua.load(script);
    let result: LuaValue = chunk.eval().map_err(|e| e.to_string())?;

    // Map result to Response
    if let LuaValue::Table(res_table) = result {
        let status: u16 = res_table.get("status").unwrap_or(200);
        let mut builder =
            Response::builder().status(StatusCode::from_u16(status).unwrap_or(StatusCode::OK));

        if let Ok(headers_table) = res_table.get::<_, Table>("headers") {
            for (k, v) in headers_table.pairs::<String, String>().flatten() {
                builder = builder.header(k, v);
            }
        }

        let body_bytes = if let Ok(body_val) = res_table.get::<_, LuaValue>("body") {
            match body_val {
                LuaValue::String(s) => s.as_bytes().to_vec(),
                LuaValue::Table(t) => {
                    let json_val: Value = lua
                        .from_value(LuaValue::Table(t))
                        .map_err(|e| e.to_string())?;
                    serde_json::to_vec(&json_val).unwrap_or_default()
                }
                _ => vec![],
            }
        } else {
            vec![]
        };

        Ok(builder.body(Body::from(body_bytes)).unwrap_or_else(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to build response",
            )
                .into_response()
        }))
    } else {
        Err("Script must return a table".to_string())
    }
}

async fn handle_request(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let start = std::time::Instant::now();
    let (parts, body) = req.into_parts();
    let path = parts.uri.path();
    let method = &parts.method;
    let headers = &parts.headers;

    let body_bytes = body
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let body_json: Option<Value> = serde_json::from_slice(&body_bytes).ok();

    tracing::info!("Incoming request: {} {}", method, path);

    let expectations = state.expectations.load();
    let matched = expectations
        .iter()
        .find(|exp| matcher::matches(method, path, headers, &body_json, &exp.condition));

    if let Some(_exp) = matched {
        metrics::REQUEST_COUNTER
            .with_label_values(&["true", path])
            .inc();
    } else {
        metrics::REQUEST_COUNTER
            .with_label_values(&["false", path])
            .inc();
    }

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

        if let Some(ref script) = exp.response.script {
            tracing::info!("Executing Lua script for mock {}", exp.id);
            match execute_lua_script(script, method.as_str(), path, headers, &body_json).await {
                Ok(res) => return res,
                Err(e) => {
                    tracing::error!("Lua execution failed: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Lua error: {}", e),
                    )
                        .into_response();
                }
            }
        }

        if let Some(ref proxy_config) = exp.response.proxy {
            tracing::info!("Proxying request to: {}", proxy_config.url);
            return forward_to_upstream(
                &state,
                proxy_config,
                parts.method,
                parts.uri,
                parts.headers,
                body_bytes,
            )
            .await;
        }

        if let Some(latency) = exp.response.response.latency
            && latency > 0
        {
            tracing::info!("Applying latency delay: {}ms", latency);
            tokio::time::sleep(std::time::Duration::from_millis(latency)).await;
        }

        if let Some(jitter_res) = apply_jitter(&exp.response, path, &body_json, headers).await {
            return jitter_res;
        }

        let status = StatusCode::from_u16(exp.response.response.status_code.unwrap_or(200))
            .unwrap_or(StatusCode::OK);
        let mut response_builder = Response::builder().status(status);

        if let Some(ref res_headers) = exp.response.response.headers {
            for (key, value) in res_headers {
                response_builder = response_builder.header(key, value);
            }
        }

        let response_body = build_response_body(
            &exp.response.response,
            path,
            &body_json,
            headers,
            &mut response_builder,
        );

        let response = response_builder.body(response_body).unwrap();
        tracing::info!("Returning matched response: status={}", response.status());

        metrics::REQUEST_DURATION
            .with_label_values(&[path])
            .observe(start.elapsed().as_secs_f64());
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

async fn forward_to_upstream(
    state: &AppState,
    proxy_config: &models::ProxyConfig,
    method: http::Method,
    uri: http::Uri,
    mut headers: HeaderMap,
    body_bytes: Bytes,
) -> Response {
    let path_and_query = uri
        .path_and_query()
        .map(|pq: &http::uri::PathAndQuery| pq.as_str())
        .unwrap_or("");

    let url = format!(
        "{}{}",
        proxy_config.url.trim_end_matches('/'),
        path_and_query
    );

    // Overlay override headers
    if let Some(ref overrides) = proxy_config.headers {
        for (k, v) in overrides {
            if let Ok(name) = header::HeaderName::from_bytes(k.as_bytes())
                && let Ok(value) = header::HeaderValue::from_str(v)
            {
                headers.insert(name, value);
            }
        }
    }

    let mut proxy_req = state.proxy_client.request(method, url).headers(headers);

    if !body_bytes.is_empty() {
        proxy_req = proxy_req.body(body_bytes);
    }

    match proxy_req.send().await {
        Ok(res) => {
            let status = StatusCode::from_u16(res.status().as_u16()).unwrap_or(StatusCode::OK);
            let mut response_builder = Response::builder().status(status);

            for (name, value) in res.headers() {
                response_builder = response_builder.header(name, value);
            }

            let body_bytes = match res.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("Failed to read upstream response body: {}", e);
                    return (
                        StatusCode::BAD_GATEWAY,
                        "Failed to read upstream response body",
                    )
                        .into_response();
                }
            };

            response_builder.body(Body::from(body_bytes)).unwrap()
        }
        Err(e) => {
            tracing::error!("Proxy request failed: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                format!("Proxy request failed: {}", e),
            )
                .into_response()
        }
    }
}

async fn apply_jitter(
    res_config: &models::MockResponse,
    path: &str,
    body_json: &Option<Value>,
    request_headers: &header::HeaderMap,
) -> Option<Response> {
    let jitter = res_config.jitter.as_ref()?;
    let random: f64 = rand::random();

    if random < jitter.probability {
        tracing::info!("Jitter matched! Returning error response");

        if let Some(latency) = jitter.response.latency
            && latency > 0
        {
            tracing::info!("Applying jitter latency delay: {}ms", latency);
            tokio::time::sleep(std::time::Duration::from_millis(latency)).await;
        }

        let status = StatusCode::from_u16(jitter.response.status_code.unwrap_or(500))
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut response_builder = Response::builder().status(status);

        if let Some(ref res_headers) = jitter.response.headers {
            for (key, value) in res_headers {
                response_builder = response_builder.header(key, value);
            }
        }

        let body = build_response_body(
            &jitter.response,
            path,
            body_json,
            request_headers,
            &mut response_builder,
        );

        return Some(response_builder.body(body).unwrap());
    }
    None
}

fn build_response_body(
    res_config: &models::ResponseConfig,
    path: &str,
    req_body: &Option<Value>,
    request_headers: &header::HeaderMap,
    response_builder: &mut axum::http::response::Builder,
) -> Body {
    let Some(ref res_body) = res_config.body else {
        return Body::empty();
    };

    let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let resolved_val =
        templating::resolve_template_value(res_body.clone(), &path_segments, req_body);
    let resolved_body = serde_json::to_string(&resolved_val).unwrap();

    // Handle Non-JSON (Text/HTML) body type
    if let Some(ref b_type) = res_config.body_type
        && b_type == "text"
    {
        return handle_text_response(resolved_body, response_builder);
    }

    let accept_bson = request_headers
        .get(header::ACCEPT)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.contains("application/bson"))
        .unwrap_or(false);

    if accept_bson && let Some(body) = handle_bson_response(&resolved_body, response_builder) {
        return body;
    }

    // Default to JSON
    let mut b = Response::builder();
    std::mem::swap(response_builder, &mut b);
    if !b
        .headers_ref()
        .map(|h| h.contains_key(header::CONTENT_TYPE))
        .unwrap_or(false)
    {
        *response_builder = b.header(header::CONTENT_TYPE, "application/json");
    } else {
        *response_builder = b;
    }
    Body::from(resolved_body)
}

fn handle_text_response(
    resolved_body: String,
    response_builder: &mut axum::http::response::Builder,
) -> Body {
    // If it's stored as a JSON string, extract the raw content
    let raw_body = serde_json::from_str::<Value>(&resolved_body)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or(resolved_body);

    let mut b = Response::builder();
    std::mem::swap(response_builder, &mut b);

    // Set default content type to text/plain if not already set by headers
    if !b
        .headers_ref()
        .map(|h| h.contains_key(header::CONTENT_TYPE))
        .unwrap_or(false)
    {
        *response_builder = b.header(header::CONTENT_TYPE, "text/plain");
    } else {
        *response_builder = b;
    }
    Body::from(raw_body)
}

fn handle_bson_response(
    resolved_body: &str,
    response_builder: &mut axum::http::response::Builder,
) -> Option<Body> {
    let val: Value = serde_json::from_str(resolved_body).unwrap_or(Value::Null);
    if let Ok(bson_val) = bson::to_bson(&val) {
        let mut bytes = Vec::new();
        if let Some(doc) = bson_val.as_document() {
            doc.to_writer(&mut bytes).unwrap();
        } else if let bson::Bson::Array(arr) = bson_val {
            let doc = bson::doc! { "data": arr };
            doc.to_writer(&mut bytes).unwrap();
        } else {
            return None;
        }

        let mut b = Response::builder();
        std::mem::swap(response_builder, &mut b);
        *response_builder = b.header(header::CONTENT_TYPE, "application/bson");
        return Some(Body::from(bytes));
    }
    None
}

async fn metrics_handler() -> impl IntoResponse {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = Vec::new();
    let metric_families = metrics::REGISTRY.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, encoder.format_type())
        .body(Body::from(buffer))
        .unwrap()
}
