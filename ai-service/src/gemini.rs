//! Google Gemini content generation.

use serde::{Deserialize, Serialize};

const DEFAULT_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const DEFAULT_MODEL: &str = "gemini-2.0-flash";

pub fn api_url() -> String {
    std::env::var("GEMINI_API_URL")
        .unwrap_or_else(|_| DEFAULT_API_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

pub fn model() -> String {
    std::env::var("GEMINI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string())
}

pub fn document_model() -> String {
    std::env::var("GEMINI_DOCUMENT_MODEL").unwrap_or_else(|_| model())
}

pub fn api_key() -> Option<String> {
    std::env::var("GEMINI_API_KEY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.starts_with("your-"))
}

pub async fn is_configured() -> bool {
    api_key().is_some()
}

fn max_tokens(requested: Option<u32>) -> u32 {
    requested.unwrap_or_else(|| {
        std::env::var("GEMINI_MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1024)
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateRequest<'a> {
    system_instruction: SystemInstruction<'a>,
    contents: Vec<Content<'a>>,
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct SystemInstruction<'a> {
    parts: Vec<Part<'a>>,
}

#[derive(Serialize)]
struct Content<'a> {
    role: &'a str,
    parts: Vec<Part<'a>>,
}

#[derive(Serialize)]
struct Part<'a> {
    text: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    temperature: f32,
    max_output_tokens: u32,
}

#[derive(Deserialize)]
struct GenerateResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<ResponseContent>,
}

#[derive(Deserialize)]
struct ResponseContent {
    parts: Option<Vec<ResponsePart>>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: Option<ErrorDetail>,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: Option<String>,
}

pub async fn complete(
    system: &str,
    user: &str,
    max_new_tokens: Option<u32>,
) -> Result<String, String> {
    complete_with_model(&model(), system, user, max_new_tokens).await
}

pub async fn complete_for_documents(
    system: &str,
    user: &str,
    max_new_tokens: Option<u32>,
) -> Result<String, String> {
    complete_with_model(&document_model(), system, user, max_new_tokens).await
}

async fn complete_with_model(
    model_name: &str,
    system: &str,
    user: &str,
    max_new_tokens: Option<u32>,
) -> Result<String, String> {
    let key = api_key().ok_or_else(|| "GEMINI_API_KEY is not set.".to_string())?;
    let body = GenerateRequest {
        system_instruction: SystemInstruction {
            parts: vec![Part { text: system }],
        },
        contents: vec![Content {
            role: "user",
            parts: vec![Part { text: user }],
        }],
        generation_config: GenerationConfig {
            temperature: 0.2,
            max_output_tokens: max_tokens(max_new_tokens),
        },
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .post(format!(
            "{}/models/{model_name}:generateContent?key={key}",
            api_url()
        ))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Gemini request failed: {e}"))?;

    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        if let Ok(err) = serde_json::from_str::<ErrorBody>(&text) {
            if let Some(msg) = err.error.and_then(|e| e.message) {
                return Err(format!("Gemini API error: {msg}"));
            }
        }
        return Err(format!("Gemini returned {status}: {text}"));
    }

    let parsed: GenerateResponse =
        serde_json::from_str(&text).map_err(|e| format!("Invalid Gemini response: {e}"))?;
    let reply = parsed
        .candidates
        .unwrap_or_default()
        .into_iter()
        .filter_map(|candidate| candidate.content)
        .flat_map(|content| content.parts.unwrap_or_default())
        .filter_map(|part| part.text)
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string();

    if reply.is_empty() {
        Err("Gemini returned an empty response".into())
    } else {
        Ok(reply)
    }
}
