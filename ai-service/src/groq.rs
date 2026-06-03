//! Groq chat completions (OpenAI-compatible API).

use serde::{Deserialize, Serialize};

const DEFAULT_API_URL: &str = "https://api.groq.com/openai/v1";
const DEFAULT_MODEL: &str = "llama-3.1-8b-instant";

pub fn api_url() -> String {
    std::env::var("GROQ_API_URL")
        .unwrap_or_else(|_| DEFAULT_API_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

pub fn model() -> String {
    std::env::var("GROQ_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string())
}

pub fn document_model() -> String {
    std::env::var("GROQ_DOCUMENT_MODEL").unwrap_or_else(|_| model())
}

pub fn api_key() -> Option<String> {
    std::env::var("GROQ_API_KEY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.starts_with("your-"))
}

pub async fn is_configured() -> bool {
    api_key().is_some()
}

fn max_tokens(requested: Option<u32>) -> u32 {
    requested.unwrap_or_else(|| {
        std::env::var("GROQ_MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1024)
    })
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
    content: Option<String>,
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
    let key = api_key().ok_or_else(|| "GROQ_API_KEY is not set.".to_string())?;
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

    let res = client
        .post(format!("{}/chat/completions", api_url()))
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Groq request failed: {e}"))?;

    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        if let Ok(err) = serde_json::from_str::<ErrorBody>(&text) {
            if let Some(msg) = err.error.and_then(|e| e.message) {
                return Err(format!("Groq API error: {msg}"));
            }
        }
        return Err(format!("Groq returned {status}: {text}"));
    }

    let parsed: ChatResponse =
        serde_json::from_str(&text).map_err(|e| format!("Invalid Groq response: {e}"))?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content.unwrap_or_default().trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Groq returned an empty response".into())
}
