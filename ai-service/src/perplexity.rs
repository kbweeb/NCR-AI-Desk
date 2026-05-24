//! Perplexity Sonar — web-grounded answers (weather, news, live facts).

use serde::{Deserialize, Serialize};

const DEFAULT_API_URL: &str = "https://api.perplexity.ai";
const DEFAULT_MODEL: &str = "sonar";

pub fn api_url() -> String {
    std::env::var("PERPLEXITY_API_URL")
        .unwrap_or_else(|_| DEFAULT_API_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

pub fn model() -> String {
    std::env::var("PERPLEXITY_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string())
}

pub fn api_key() -> Option<String> {
    std::env::var("PERPLEXITY_API_KEY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn max_tokens(requested: Option<u32>) -> u32 {
    requested.unwrap_or_else(|| {
        std::env::var("PERPLEXITY_MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1024)
    })
}

pub async fn is_configured() -> bool {
    api_key().is_some()
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: Option<ErrorDetail>,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: Option<String>,
}

pub fn document_model() -> String {
    std::env::var("PERPLEXITY_DOCUMENT_MODEL").unwrap_or_else(|_| model())
}

pub async fn complete(system: &str, user: &str, max_new_tokens: Option<u32>) -> Result<String, String> {
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
    let key = api_key().ok_or_else(|| {
        "PERPLEXITY_API_KEY is not set. Add it to .env (get a key at https://www.perplexity.ai/settings/api)."
            .to_string()
    })?;

    let body = ChatRequest {
        model: model_name,
        messages: vec![
            ChatMessage {
                role: "system",
                content: system,
            },
            ChatMessage {
                role: "user",
                content: user,
            },
        ],
        max_tokens: max_tokens(max_new_tokens),
        temperature: 0.2,
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/chat/completions", api_url());
    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Perplexity request failed: {e}"))?;

    let status = res.status();
    let text = res.text().await.unwrap_or_default();

    if !status.is_success() {
        if let Ok(err) = serde_json::from_str::<ErrorBody>(&text) {
            if let Some(msg) = err.error.and_then(|e| e.message) {
                return Err(format!("Perplexity API error: {msg}"));
            }
        }
        return Err(format!("Perplexity returned {status}: {text}"));
    }

    let parsed: ChatResponse = serde_json::from_str(&text)
        .map_err(|e| format!("Invalid Perplexity response: {e}"))?;

    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Perplexity returned an empty response".into())
}
