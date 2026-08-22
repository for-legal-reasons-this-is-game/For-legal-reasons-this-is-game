use serde_json::{Value, json};
use uuid::Uuid;

fn base_url() -> String {
    std::env::var("BACKEND_URL").unwrap_or_else(|_| "http://127.0.0.1:8000".to_string()) + "/api/v1"
}

#[tokio::test]
async fn create_and_fetch_user() {
    let client = reqwest::Client::new();
    let base = base_url();

    let create_res = client
        .post(format!("{base}/users"))
        .json(&json!({ "name": format!("Ada Lovelace {}", Uuid::new_v4()) }))
        .send()
        .await
        .expect("create request failed");

    assert_eq!(create_res.status(), 201);
    let created: Value = create_res.json().await.expect("invalid json");
    let user_id = created["user_id"].as_str().expect("missing user_id");

    let fetch_res = client
        .get(format!("{base}/users/{user_id}"))
        .send()
        .await
        .expect("fetch request failed");

    assert_eq!(fetch_res.status(), 200);
    let fetched: Value = fetch_res.json().await.expect("invalid json");
    assert_eq!(fetched["user_id"], user_id);
    assert_eq!(fetched["user_name"], created["user_name"]);
}

#[tokio::test]
async fn list_users_includes_created_user() {
    let client = reqwest::Client::new();
    let base = base_url();
    let name = format!("Grace Hopper {}", Uuid::new_v4());

    client
        .post(format!("{base}/users"))
        .json(&json!({ "name": name }))
        .send()
        .await
        .expect("create request failed");

    let list_res = client
        .get(format!("{base}/users"))
        .send()
        .await
        .expect("list request failed");

    assert_eq!(list_res.status(), 200);
    let users: Vec<Value> = list_res.json().await.expect("invalid json");
    assert!(users.iter().any(|u| u["user_name"] == name));
}

#[tokio::test]
async fn fetch_nonexistent_user_returns_404() {
    let client = reqwest::Client::new();
    let base = base_url();
    let random_id = Uuid::new_v4();

    let res = client
        .get(format!("{base}/users/{random_id}"))
        .send()
        .await
        .expect("fetch request failed");

    assert_eq!(res.status(), 404);
}
