use serde_json::{Value, json};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

struct TestServer {
    child: Child,
}

impl TestServer {
    fn start(port: Option<u16>, expectations: Option<&str>) -> Self {
        let mut cmd = Command::new("target/debug/mimicrab");
        if let Some(p) = port {
            cmd.arg("--port").arg(p.to_string());
        }
        if let Some(e) = expectations {
            cmd.arg("--expectations").arg(e);
        }
        let child = cmd
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Failed to start mimicrab server");

        Self { child }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn wait_for_server(base_url: &str) {
    let client = reqwest::Client::new();
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(30);

    while start.elapsed() < timeout {
        if let Ok(res) = client
            .get(format!("{}/_admin/mocks", base_url))
            .send()
            .await
        {
            if res.status() == 200 {
                return;
            }
        }
        sleep(Duration::from_millis(500)).await;
    }
    panic!(
        "Server at {} did not become ready within {:?}",
        base_url, timeout
    );
}

#[tokio::test]
async fn test_full_mock_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let port = 3020;
    let _server = TestServer::start(Some(port), Some("expectations_lifecycle.json"));
    let base_url = format!("http://localhost:{}", port);
    wait_for_server(&base_url).await;

    let admin_url = format!("{}/_admin/mocks", base_url);
    let client = reqwest::Client::new();

    // 1. List mocks (initially empty or some existing)
    let res = client.get(&admin_url).send().await?;
    assert_eq!(res.status(), 200);
    let initial_mocks: Vec<Value> = res.json().await?;
    initial_mocks.len();

    // 2. Add a mock
    let new_mock = json!({
        "id": 999123,
        "condition": {
            "method": "GET",
            "path": "/test-integration"
        },
        "response": {
            "status_code": 201,
            "body": { "result": "ok" }
        }
    });

    let res = client.post(&admin_url).json(&new_mock).send().await?;
    assert_eq!(res.status(), 201);

    // 3. Verify it works
    let res = client
        .get(format!("{}/test-integration", base_url))
        .send()
        .await?;
    assert_eq!(res.status(), 201);
    let body: Value = res.json().await?;
    assert_eq!(body["result"], "ok");

    // 4. Update the mock
    let updated_mock = json!({
        "id": 999123,
        "condition": {
            "method": "GET",
            "path": "/test-integration"
        },
        "response": {
            "status_code": 200,
            "body": { "result": "updated" }
        }
    });

    let res = client
        .put(format!("{}/{}", admin_url, 999123))
        .json(&updated_mock)
        .send()
        .await?;
    assert_eq!(res.status(), 200);

    // 5. Verify update
    let res = client
        .get(format!("{}/test-integration", base_url))
        .send()
        .await?;
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await?;
    assert_eq!(body["result"], "updated");

    // 6. Test Export
    let res = client
        .get(format!("{}/_admin/export", base_url))
        .send()
        .await?;
    assert_eq!(res.status(), 200);
    let export: Vec<Value> = res.json().await?;
    assert!(export.iter().any(|m| m["id"] == 999123));

    // 7. Delete mock
    let res = client
        .delete(format!("{}/{}", admin_url, 999123))
        .send()
        .await?;
    assert_eq!(res.status(), 204);

    // 8. Verify deletion (should return 404 from mock handler now)
    let res = client
        .get(format!("{}/test-integration", base_url))
        .send()
        .await?;
    assert_eq!(res.status(), 404);

    Ok(())
}

#[tokio::test]
async fn test_import_mocks() -> Result<(), Box<dyn std::error::Error>> {
    let port = 3021;
    let _server = TestServer::start(Some(port), Some("expectations_import.json"));
    let base_url = format!("http://localhost:{}", port);
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let import_url = format!("{}/_admin/import", base_url);

    let mocks_to_import = json!([
        {
            "id": 8881,
            "condition": { "method": "GET", "path": "/import-1" },
            "response": { "status_code": 200, "body": { "from": "import" } }
        },
        {
            "id": 8882,
            "condition": { "method": "POST", "path": "/import-2" },
            "response": { "status_code": 201, "body": { "msg": "created" } }
        }
    ]);

    let res = client
        .post(&import_url)
        .json(&mocks_to_import)
        .send()
        .await?;
    assert_eq!(res.status(), 200);

    // Verify imported mocks
    let res = client.get(format!("{}/import-1", base_url)).send().await?;
    assert_eq!(res.status(), 200);

    let res = client.post(format!("{}/import-2", base_url)).send().await?;
    assert_eq!(res.status(), 201);

    Ok(())
}

#[tokio::test]
async fn test_mock_latency() -> Result<(), Box<dyn std::error::Error>> {
    let port = 3022;
    let _server = TestServer::start(Some(port), Some("expectations_latency.json"));
    let base_url = format!("http://localhost:{}", port);
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let admin_url = format!("{}/_admin/mocks", base_url);

    // Add a mock with 500ms latency
    let latency_ms = 500;
    let new_mock = json!({
        "id": 777123,
        "condition": {
            "method": "GET",
            "path": "/latency-test"
        },
        "response": {
            "status_code": 200,
            "body": { "delayed": true },
            "latency": latency_ms
        }
    });

    client.post(&admin_url).json(&new_mock).send().await?;

    // Measure request duration
    let start = std::time::Instant::now();
    let res = client
        .get(format!("{}/latency-test", base_url))
        .send()
        .await?;
    let duration = start.elapsed();

    assert_eq!(res.status(), 200);
    assert!(
        duration >= Duration::from_millis(latency_ms),
        "Response was too fast: {:?}",
        duration
    );

    Ok(())
}

#[tokio::test]
async fn test_mock_jitter() -> Result<(), Box<dyn std::error::Error>> {
    let port = 3023;
    let _server = TestServer::start(Some(port), Some("expectations_jitter.json"));
    let base_url = format!("http://localhost:{}", port);
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let admin_url = format!("{}/_admin/mocks", base_url);

    // Add a mock with 100% jitter (probability 1.0)
    let new_mock = json!({
        "id": 555123,
        "condition": {
            "method": "GET",
            "path": "/jitter-test"
        },
        "response": {
            "status_code": 200,
            "body": { "ok": true },
            "jitter": {
                "probability": 1.0,
                "status_code": 503,
                "body": { "error": "service unavailable" }
            }
        }
    });

    client.post(&admin_url).json(&new_mock).send().await?;

    // Verify it returns the jitter response (100% probability)
    let res = client
        .get(format!("{}/jitter-test", base_url))
        .send()
        .await?;
    assert_eq!(res.status(), 503);
    let body: Value = res.json().await?;
    assert_eq!(body["error"], "service unavailable");

    // Update to 0% jitter
    let updated_mock = json!({
        "id": 555123,
        "condition": {
            "method": "GET",
            "path": "/jitter-test"
        },
        "response": {
            "status_code": 200,
            "body": { "ok": true },
            "jitter": {
                "probability": 0.0,
                "status_code": 503,
                "body": { "error": "service unavailable" }
            }
        }
    });

    client
        .put(format!("{}/{}", admin_url, 555123))
        .json(&updated_mock)
        .send()
        .await?;

    // Verify it returns the normal response (0% probability)
    let res = client
        .get(format!("{}/jitter-test", base_url))
        .send()
        .await?;
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await?;
    assert_eq!(body["ok"], true);

    Ok(())
}

#[tokio::test]
async fn test_mock_proxy() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Start Upstream on 3001
    let _upstream = TestServer::start(Some(3001), Some("expectations_upstream.json"));
    wait_for_server("http://localhost:3001").await;

    // 2. Start Primary on 3002
    let _primary = TestServer::start(Some(3002), Some("expectations_primary.json"));
    wait_for_server("http://localhost:3002").await;

    let client = reqwest::Client::new();

    // 3. Configure Upstream to return a specific response
    client
        .post("http://localhost:3001/_admin/mocks")
        .json(&json!({
            "id": 1,
            "condition": { "method": "GET", "path": "/proxy-test" },
            "response": { "status_code": 200, "body": { "from": "upstream" } }
        }))
        .send()
        .await?;

    // 4. Configure Primary to proxy to Upstream with header override
    client
        .post("http://localhost:3002/_admin/mocks")
        .json(&json!({
            "id": 2,
            "condition": { "method": "GET", "path": "/proxy-test" },
            "response": {
                "proxy": {
                    "url": "http://localhost:3001",
                    "headers": { "X-Proxy-Overridden": "true", "User-Agent": "mimicrab-test" }
                }
            }
        }))
        .send()
        .await?;

    // 5. Verify proxying works
    let res = client
        .get("http://localhost:3002/proxy-test")
        .header("User-Agent", "original-agent")
        .send()
        .await?;

    assert_eq!(res.status(), 200);
    let body: Value = res.json().await?;
    assert_eq!(body["from"], "upstream");

    Ok(())
}

#[tokio::test]
async fn test_static_asset_caching_and_compression() -> Result<(), Box<dyn std::error::Error>> {
    let _server = TestServer::start(Some(3003), Some("expectations_static.json"));
    wait_for_server("http://localhost:3003").await;

    let client = reqwest::Client::new();

    // 1. Test ETag / 304 Not Modified
    let res = client
        .get("http://localhost:3003/ui/index.html")
        .send()
        .await?;
    assert_eq!(res.status(), 200);
    let etag = res
        .headers()
        .get("etag")
        .expect("ETag header missing")
        .clone();

    let res304 = client
        .get("http://localhost:3003/ui/index.html")
        .header("If-None-Match", etag)
        .send()
        .await?;
    assert_eq!(res304.status(), 304);

    /*
      // 2. Test Brotli Preference
      let res_br = client
          .get("http://localhost:3003/ui/index.html")
          .header("Accept-Encoding", "br, gzip")
          .send()
          .await?;
      assert_eq!(res_br.status(), 200);
      // Note: rust-embed-for-web might not have br for very small files if skipped,
      // but index.html should be large enough or we can check what's actually there.
      let enc = res_br.headers().get("content-encoding");
      assert!(enc.is_some(), "Content-Encoding should be present for br");
      assert_eq!(enc.unwrap(), "br");

      // 3. Test Gzip Fallback
      let res_gzip = client
          .get("http://localhost:3003/ui/index.html")
          .header("Accept-Encoding", "gzip")
          .send()
          .await?;
      assert_eq!(res_gzip.status(), 200);
      let enc = res_gzip.headers().get("content-encoding");
      assert!(enc.is_some(), "Content-Encoding should be present for gzip");
      assert_eq!(enc.unwrap(), "gzip");
    */
    // 4. Test Uncompressed
    let res_none = client
        .get("http://localhost:3003/ui/index.html")
        .header("Accept-Encoding", "identity")
        .send()
        .await?;
    assert_eq!(res_none.status(), 200);
    assert!(res_none.headers().get("content-encoding").is_none());

    Ok(())
}

#[tokio::test]
async fn test_mock_with_lua_script() -> Result<(), Box<dyn std::error::Error>> {
    let _server = TestServer::start(Some(3004), Some("expectations_lua.json"));
    wait_for_server("http://localhost:3004").await;

    let client = reqwest::Client::new();

    // Configure mock with Lua script
    let script = r#"
        local res = {
            status = 201,
            headers = {
                ["X-Lua-Generated"] = "true",
                ["Content-Type"] = "application/json"
            },
            body = {
                received_method = request.method,
                received_path = request.path,
                received_header = request.headers["x-test-header"],
                received_body = request.body
            }
        }
        return res
    "#;

    client
        .post("http://localhost:3004/_admin/mocks")
        .json(&json!({
            "id": 1,
            "condition": { "method": "POST", "path": "/lua-test" },
            "response": {
                "script": script
            }
        }))
        .send()
        .await?;

    // Test the mock
    let res = client
        .post("http://localhost:3004/lua-test")
        .header("X-Test-Header", "lua-val")
        .json(&json!({ "input": "data" }))
        .send()
        .await?;

    assert_eq!(res.status(), 201);
    assert_eq!(res.headers().get("X-Lua-Generated").unwrap(), "true");

    let body: Value = res.json().await?;
    assert_eq!(body["received_method"], "POST");
    assert_eq!(body["received_path"], "/lua-test");
    assert_eq!(body["received_header"], "lua-val");
    assert_eq!(body["received_body"]["input"], "data");

    Ok(())
}

#[tokio::test]
async fn test_templated_body_array() -> Result<(), Box<dyn std::error::Error>> {
    let _server = TestServer::start(Some(3005), Some("expectations_templated_array.json"));
    wait_for_server("http://localhost:3005").await;

    let client = reqwest::Client::new();

    // Create a mock that uses body array access
    client
        .post("http://localhost:3005/_admin/mocks")
        .json(&json!({
            "id": 1,
            "condition": { "method": "POST", "path": "/array-test" },
            "response": {
                "status_code": 200,
                "body": {
                    "first_name": "{{body[0].name}}",
                    "second_id": "{{body[1].id}}",
                    "deep": "{{body[1].tags[0]}}",
                    "path_0": "{{path[0]}}"
                }
            }
        }))
        .send()
        .await?;

    // Test with array in body
    let res = client
        .post("http://localhost:3005/array-test")
        .json(&json!([
            { "id": 101, "name": "Alice" },
            { "id": 102, "name": "Bob", "tags": ["tag1", "tag2"] }
        ]))
        .send()
        .await?;

    assert_eq!(res.status(), 200);
    let body: Value = res.json().await?;
    assert_eq!(body["first_name"], "Alice");
    assert_eq!(body["second_id"], "102");
    assert_eq!(body["deep"], "tag1");
    assert_eq!(body["path_0"], "array-test");

    Ok(())
}

#[tokio::test]
async fn test_metrics() -> Result<(), Box<dyn std::error::Error>> {
    let _server = TestServer::start(Some(3006), Some("expectations_metrics.json"));
    wait_for_server("http://localhost:3006").await;

    let client = reqwest::Client::new();
    let base_url = "http://localhost:3006";

    // 1. Create a mock
    client
        .post(format!("{}/_admin/mocks", base_url))
        .json(&json!({
            "id": 888,
            "condition": { "method": "GET", "path": "/metrics-test" },
            "response": { "status_code": 200, "body": "ok" }
        }))
        .send()
        .await?;

    // 2. Call the mock
    client
        .get(format!("{}/metrics-test", base_url))
        .send()
        .await?;

    // 3. Call a non-existent path unsing a method
    client.get(format!("{}/no-match", base_url)).send().await?;

    // 4. Verify metrics
    let res = client
        .get(format!("{}/_admin/metrics", base_url))
        .send()
        .await?;
    assert_eq!(res.status(), 200);
    let body = res.text().await?;

    assert!(body.contains("mimicrab_requests_total{matched=\"true\",path=\"/metrics-test\"} 1"));
    assert!(body.contains("mimicrab_requests_total{matched=\"false\",path=\"/no-match\"} 1"));
    assert!(body.contains("mimicrab_request_duration_seconds_bucket{path=\"/metrics-test\""));

    // Check process metrics exist
    assert!(body.contains("process_cpu_seconds_total"));
    assert!(body.contains("process_resident_memory_bytes"));

    Ok(())
}
