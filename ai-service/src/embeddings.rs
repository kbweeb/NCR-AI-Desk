//! Text embeddings via the Python inference service (fastembed).

use serde::{Deserialize, Serialize};

pub const VECTOR_SIZE: usize = 384;

pub fn embed_service_url() -> String {
    if let Ok(url) = std::env::var("EMBED_SERVICE_URL") {
        if !url.is_empty() {
            return url;
        }
    }
    let base = std::env::var("DOCUMENT_SERVICE_URL")
        .or_else(|_| std::env::var("QWEN_INFERENCE_URL"))
        .unwrap_or_else(|_| "http://127.0.0.1:8092".to_string());
    format!("{}/embed", base.trim_end_matches('/'))
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    texts: Vec<&'a str>,
}

#[derive(Deserialize)]
struct EmbedResponse {
    vectors: Vec<Vec<f32>>,
}

pub async fn embed_text(text: &str) -> Result<Vec<f32>, String> {
    let vectors = embed_batch(&[text]).await?;
    vectors
        .into_iter()
        .next()
        .ok_or_else(|| "Embed service returned no vectors".into())
}

pub async fn embed_batch(texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
    if texts.is_empty() {
        return Ok(vec![]);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .post(embed_service_url())
        .json(&EmbedRequest { texts: texts.to_vec() })
        .send()
        .await
        .map_err(|e| format!("Embed request failed (is the document service on 8092 running?): {e}"))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Embed service returned {status}: {body}"));
    }

    let parsed: EmbedResponse = res.json().await.map_err(|e| e.to_string())?;
    if parsed.vectors.len() != texts.len() {
        return Err("Embed service returned unexpected vector count".into());
    }
    Ok(parsed.vectors)
}

pub async fn embed_service_available() -> bool {
    let embed_url = embed_service_url();
    let base = embed_url.trim_end_matches("/embed");
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    client
        .get(format!("{base}/health"))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
