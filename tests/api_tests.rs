use serde_json::{Value, json};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

struct TestServer {
    child: Child,
}

impl TestServer {
    fn start() -> Self {
        let child = Command::new("target/debug/mimicrab")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
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
    let _server = TestServer::start();
    let base_url = "http://localhost:3000";
    wait_for_server(base_url).await;

    let client = reqwest::Client::new();
    let base_url = "http://localhost:3000";
    let admin_url = format!("{}/_admin/mocks", base_url);

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
    let _server = TestServer::start();
    let base_url = "http://localhost:3000";
    wait_for_server(base_url).await;

    let client = reqwest::Client::new();
    let base_url = "http://localhost:3000";
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
