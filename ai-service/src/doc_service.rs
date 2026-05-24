//! Document extract/export + embeddings (Python service on port 8092).

use serde::Deserialize;

pub fn base_url() -> String {
    std::env::var("DOCUMENT_SERVICE_URL")
        .or_else(|_| std::env::var("QWEN_INFERENCE_URL"))
        .unwrap_or_else(|_| "http://127.0.0.1:8092".to_string())
        .trim_end_matches('/')
        .to_string()
}

#[derive(Deserialize)]
struct HealthResponse {
    ready: Option<bool>,
    status: Option<String>,
}

pub async fn is_available() -> bool {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    let url = format!("{}/health", base_url());
    let Ok(res) = client.get(&url).send().await else {
        return false;
    };
    if !res.status().is_success() {
        return false;
    }
    let Ok(body) = res.json::<HealthResponse>().await else {
        return false;
    };
    body.ready.unwrap_or_else(|| body.status.as_deref() == Some("ok"))
}
