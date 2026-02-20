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
async fn test_typed_templates() -> Result<(), Box<dyn std::error::Error>> {
    let port = 3015;
    let _server = TestServer::start(port, "expectations_typed.json");
    let base_url = format!("http://localhost:{}", port);
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let admin_url = format!("{}/_admin/mocks", base_url);

    // 1. Numeric path segment
    client
        .post(&admin_url)
        .json(&json!({
            "id": 1,
            "condition": { "method": "GET", "path": "/user/:id" },
            "response": { "status_code": 200, "body": { "id": "{{path[1]}}", "type": "id" } }
        }))
        .send()
        .await?;

    // 2. Boolean and string conversion filter
    client
        .post(&admin_url)
        .json(&json!({
            "id": 2,
            "condition": { "method": "GET", "path": "/convert/:val" },
            "response": {
                "status_code": 200,
                "body": {
                    "original": "{{path[1]}}",
                    "as_string": "{{path[1]:string}}"
                }
            }
        }))
        .send()
        .await?;

    // 3. Typed body fields
    client
        .post(&admin_url)
        .json(&json!({
            "id": 3,
            "condition": { "method": "POST", "path": "/echo" },
            "response": {
                "status_code": 200,
                "body": {
                    "number": "{{body.num}}",
                    "bool": "{{body.flag}}",
                    "obj": "{{body.data}}",
                    "num_str": "{{body.num:string}}",
                    "flag_str": "{{body.flag:string}}"
                }
            }
        }))
        .send()
        .await?;

    // Verify numeric path
    let res = client.get(format!("{}/user/123", base_url)).send().await?;
    let body: serde_json::Value = res.json().await?;
    assert!(body["id"].is_number());
    assert_eq!(body["id"], 123);

    // Verify boolean and string filter
    let res = client
        .get(format!("{}/convert/true", base_url))
        .send()
        .await?;
    let body: serde_json::Value = res.json().await?;
    assert!(body["original"].is_boolean());
    assert_eq!(body["original"], true);
    assert!(body["as_string"].is_string());
    assert_eq!(body["as_string"], "true");

    // Verify typed body and nested objects
    let echo_payload = json!({
        "num": 42,
        "flag": false,
        "data": { "nested": "val" }
    });
    let res = client
        .post(format!("{}/echo", base_url))
        .json(&echo_payload)
        .send()
        .await?;
    let body: serde_json::Value = res.json().await?;
    assert!(body["number"].is_number());
    assert_eq!(body["number"], 42);
    assert!(body["bool"].is_boolean());
    assert_eq!(body["bool"], false);
    assert!(body["obj"].is_object());
    assert_eq!(body["obj"]["nested"], "val");
    assert_eq!(body["num_str"], "42");
    assert!(body["flag_str"].is_string());
    assert_eq!(body["flag_str"], "false");

    // 4. Explicit conversions (String to Int/Bool)
    client
        .post(&admin_url)
        .json(&json!({
            "id": 4,
            "condition": { "method": "POST", "path": "/convert-types/:val" },
            "response": {
                "status_code": 200,
                "body": {
                    "id_as_int": "{{body.id_str:int}}",
                    "active_as_bool": "{{body.active_str:bool}}",
                    "path_idx_as_int": "{{path[1]:int}}"
                }
            }
        }))
        .send()
        .await?;

    let convert_payload = json!({
        "id_str": "999",
        "active_str": "true"
    });
    // Use /convert/123 to get "123" at path[1]
    let res = client
        .post(format!("{}/convert-types/123", base_url))
        .json(&convert_payload)
        .send()
        .await?;
    let body: serde_json::Value = res.json().await?;
    assert!(body["id_as_int"].is_number());
    assert_eq!(body["id_as_int"], 999);
    assert!(body["active_as_bool"].is_boolean());
    assert_eq!(body["active_as_bool"], true);
    assert!(body["path_idx_as_int"].is_number());
    assert_eq!(body["path_idx_as_int"], 123);

    Ok(())
}
