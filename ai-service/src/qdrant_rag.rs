//! Qdrant Cloud RAG — vector search over the NCR knowledge base.

use crate::embeddings::{self, VECTOR_SIZE};
use crate::kb::{entries, KbEntry}; // KbEntry used in return type
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn qdrant_url() -> Option<String> {
    std::env::var("QDRANT_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .map(|s| normalize_qdrant_url(&s))
}

/// Cluster root only — no `/collections/...` path. Adds :6333 for Qdrant Cloud when omitted.
fn normalize_qdrant_url(raw: &str) -> String {
    let mut base = raw.trim().trim_end_matches('/').to_string();
    if let Some(idx) = base.find("/collections") {
        base.truncate(idx);
        base = base.trim_end_matches('/').to_string();
    }
    if base.contains("cloud.qdrant.io") {
        let scheme_end = base.find("://").map(|i| i + 3).unwrap_or(0);
        let rest = &base[scheme_end..];
        let path_start = rest.find('/').unwrap_or(rest.len());
        let host_port = &rest[..path_start];
        if !host_port.contains(':') {
            base = format!(
                "{}{}:6333{}",
                &base[..scheme_end],
                host_port,
                &rest[path_start..]
            );
        }
    }
    base
}

pub fn qdrant_api_key() -> Option<String> {
    std::env::var("QDRANT_API_KEY").ok().filter(|s| !s.is_empty())
}

pub fn collection_name() -> String {
    std::env::var("QDRANT_COLLECTION").unwrap_or_else(|_| "ncr-desk-kb".into())
}

pub fn is_configured() -> bool {
    qdrant_url().is_some() && qdrant_api_key().is_some()
}

fn point_id_for(entry_id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    entry_id.hash(&mut hasher);
    hasher.finish()
}

fn entry_embed_text(entry: &KbEntry) -> String {
    format!(
        "{}\n{}\n{}\n{}",
        entry.title,
        entry.body.replace("**", ""),
        entry.category,
        entry.tags.join(" ")
    )
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .connect_timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

fn api_url(path: &str) -> Result<String, String> {
    let base = qdrant_url().ok_or_else(|| "QDRANT_URL not set".to_string())?;
    let base = base.trim_end_matches('/');
    Ok(format!("{base}{path}"))
}

async fn qdrant_request(
    method: reqwest::Method,
    path: &str,
    body: Option<impl Serialize>,
) -> Result<reqwest::Response, String> {
    let url = api_url(path)?;
    let key = qdrant_api_key().ok_or_else(|| "QDRANT_API_KEY not set".to_string())?;
    let http = client()?;

    let mut req = http
        .request(method, &url)
        .header("api-key", &key)
        .header("Authorization", format!("Bearer {key}"))
        .header("Content-Type", "application/json");

    if let Some(b) = body {
        req = req.json(&b);
    }

    req.send().await.map_err(|e| {
        format!(
            "Qdrant request failed ({url}): {e}. Check QDRANT_URL (cluster root only), \
             QDRANT_API_KEY, and outbound HTTPS to cloud.qdrant.io."
        )
    })
}

#[derive(Serialize)]
struct CreateCollectionBody {
    vectors: VectorConfig,
}

#[derive(Serialize)]
struct VectorConfig {
    size: usize,
    distance: &'static str,
}

pub async fn ensure_collection() -> Result<(), String> {
    let name = collection_name();
    let path = format!("/collections/{name}");

    for attempt in 1..=3 {
        match qdrant_request(reqwest::Method::GET, &path, None::<()>).await {
            Ok(res) if res.status().is_success() => return Ok(()),
            Ok(res) if res.status().as_u16() == 404 => break,
            Ok(_) => break,
            Err(e) if attempt < 3 => {
                eprintln!("  RAG: Qdrant check retry {attempt}/3: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e),
        }
    }

    let body = CreateCollectionBody {
        vectors: VectorConfig {
            size: VECTOR_SIZE,
            distance: "Cosine",
        },
    };

    let res = qdrant_request(reqwest::Method::PUT, &path, Some(body)).await?;
    if res.status().is_success() || res.status().as_u16() == 409 {
        return Ok(());
    }
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    Err(format!("Failed to create collection: {status} {text}"))
}

#[derive(Serialize)]
struct QdrantPoint {
    id: u64,
    vector: Vec<f32>,
    payload: PointPayload,
}

#[derive(Serialize, Deserialize)]
struct PointPayload {
    id: String,
    title: String,
    body: String,
    category: String,
}

#[derive(Serialize)]
struct UpsertBody {
    points: Vec<QdrantPoint>,
}

pub async fn index_kb() -> Result<usize, String> {
    ensure_collection().await?;

    let kb = entries();
    let texts: Vec<String> = kb.iter().map(entry_embed_text).collect();
    let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let vectors = embeddings::embed_batch(&text_refs).await?;

    let points: Vec<QdrantPoint> = kb
        .iter()
        .zip(vectors.iter())
        .map(|(entry, vector)| QdrantPoint {
            id: point_id_for(entry.id),
            vector: vector.clone(),
            payload: PointPayload {
                id: entry.id.to_string(),
                title: entry.title.to_string(),
                body: entry.body.to_string(),
                category: entry.category.to_string(),
            },
        })
        .collect();

    let count = points.len();
    let name = collection_name();
    let path = format!("/collections/{name}/points?wait=true");

    let res = qdrant_request(reqwest::Method::PUT, &path, Some(UpsertBody { points })).await?;
    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("Qdrant upsert failed: {status} {text}"));
    }

    Ok(count)
}

#[derive(Serialize)]
struct SearchBody {
    vector: Vec<f32>,
    limit: usize,
    with_payload: bool,
}

#[derive(Deserialize)]
struct SearchResponse {
    result: Vec<ScoredPoint>,
}

#[derive(Deserialize)]
struct ScoredPoint {
    score: f32,
    payload: Option<PointPayload>,
}

pub async fn search(query: &str, limit: usize) -> Result<Vec<(KbEntry, f32)>, String> {
    if !is_configured() {
        return Err("Qdrant not configured".into());
    }

    let vector = embeddings::embed_text(query).await?;
    let name = collection_name();
    let path = format!("/collections/{name}/points/search");

    let body = SearchBody {
        vector,
        limit,
        with_payload: true,
    };

    let res = qdrant_request(reqwest::Method::POST, &path, Some(body)).await?;
    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("Qdrant search failed: {status} {text}"));
    }

    let parsed: SearchResponse = res.json().await.map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for hit in parsed.result {
        let Some(payload) = hit.payload else {
            continue;
        };
        if let Some(entry) = crate::kb::entry_by_id(&payload.id) {
            out.push((entry, hit.score));
        }
    }

    Ok(out)
}

/// Startup sync when Qdrant + embed service are available.
pub async fn sync_if_ready() {
    if !is_configured() {
        println!("  RAG: Qdrant not configured (set QDRANT_URL + QDRANT_API_KEY in .env)");
        return;
    }
    if !embeddings::embed_service_available().await {
        println!("  RAG: Qdrant configured but embed service offline — index after Qwen starts");
        return;
    }
    for attempt in 1..=3 {
        match index_kb().await {
            Ok(n) => {
                println!("  RAG: indexed {n} KB entries in Qdrant ({})", collection_name());
                return;
            }
            Err(e) => {
                eprintln!("  RAG: index attempt {attempt}/3 failed: {e}");
                if attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            }
        }
    }
    eprintln!("  RAG: indexing gave up — directory search still works (local KB).");
}

pub async fn is_ready() -> bool {
    is_configured() && embeddings::embed_service_available().await
}
