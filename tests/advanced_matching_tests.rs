use serde_json::json;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

struct TestServer {
    child: Child,
    expectations_path: String,
}

impl TestServer {
    fn start(port: u16, expectations_path: &str) -> Self {
        let mut cmd = Command::new("target/debug/mimicrab");
        cmd.arg("--port")
            .arg(port.to_string())
            .arg("--expectations")
            .arg(expectations_path);
        let child = cmd
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Failed to start mimicrab server");

        Self {
            child,
            expectations_path: expectations_path.to_string(),
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.expectations_path);
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
async fn test_parameterized_path_matching() -> Result<(), Box<dyn std::error::Error>> {
    let port = 3010;
    let _server = TestServer::start(port, "expectations_param.json");
    let base_url = format!("http://localhost:{}", port);
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let admin_url = format!("{}/_admin/mocks", base_url);

    // 1. Add mock with parameter
    client.post(&admin_url).json(&json!({
        "id": 1,
        "condition": { "method": "GET", "path": "/books/:id/author" },
        "response": { "status_code": 200, "body": { "matched": "parameter", "id": "{{path[1]:string}}" } }
    })).send().await?;

    // 2. Verify match
    let res = client
        .get(format!("{}/books/123/author", base_url))
        .send()
        .await?;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await?;
    assert_eq!(body["id"], "123");

    let res = client
        .get(format!("{}/books/abc/author", base_url))
        .send()
        .await?;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await?;
    assert_eq!(body["id"], "abc");

    // 3. Verify no match on different structure
    let res = client
        .get(format!("{}/books/123/wrong", base_url))
        .send()
        .await?;
    assert_eq!(res.status(), 404);

    Ok(())
}

#[tokio::test]
async fn test_wildcard_path_matching() -> Result<(), Box<dyn std::error::Error>> {
    let port = 3011;
    let _server = TestServer::start(port, "expectations_wildcard.json");
    let base_url = format!("http://localhost:{}", port);
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let admin_url = format!("{}/_admin/mocks", base_url);

    // 1. Prefix wildcard
    client
        .post(&admin_url)
        .json(&json!({
            "id": 1,
            "condition": { "method": "GET", "path": "*/books" },
            "response": { "status_code": 200, "body": { "matched": "prefix-wildcard" } }
        }))
        .send()
        .await?;

    // 2. Suffix wildcard
    client
        .post(&admin_url)
        .json(&json!({
            "id": 2,
            "condition": { "method": "GET", "path": "/api/*" },
            "response": { "status_code": 200, "body": { "matched": "suffix-wildcard" } }
        }))
        .send()
        .await?;

    // 3. Middle wildcard (not explicitly requested but good to have)
    client
        .post(&admin_url)
        .json(&json!({
            "id": 3,
            "condition": { "method": "GET", "path": "/static/*/main.js" },
            "response": { "status_code": 200, "body": { "matched": "middle-wildcard" } }
        }))
        .send()
        .await?;

    // Verify prefix wildcard
    let res = client
        .get(format!("{}/path/to/books", base_url))
        .send()
        .await?;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await?;
    assert_eq!(body["matched"], "prefix-wildcard");

    // Verify suffix wildcard
    let res = client
        .get(format!("{}/api/v1/users", base_url))
        .send()
        .await?;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await?;
    assert_eq!(body["matched"], "suffix-wildcard");

    // Verify middle wildcard
    let res = client
        .get(format!("{}/static/v1.2.3/main.js", base_url))
        .send()
        .await?;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await?;
    assert_eq!(body["matched"], "middle-wildcard");

    Ok(())
}
