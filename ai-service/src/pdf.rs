//! Document extract/export via the Python document service.

use crate::doc_service;
use serde::Deserialize;

#[derive(Deserialize)]
struct ExtractResponse {
    text: String,
    #[serde(rename = "pageCount")]
    page_count: u32,
    #[serde(rename = "charCount")]
    char_count: usize,
    #[serde(default)]
    format: String,
}

pub struct ExtractedDocument {
    pub text: String,
    pub page_count: u32,
    pub char_count: usize,
    pub format: String,
}

pub struct ExportedDocument {
    pub bytes: Vec<u8>,
    pub filename: String,
    pub mime_type: String,
}

pub async fn extract_document_bytes(
    data: &[u8],
    filename: &str,
) -> Result<ExtractedDocument, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| e.to_string())?;

    let part = reqwest::multipart::Part::bytes(data.to_vec())
        .file_name(filename.to_string())
        .mime_str(guess_mime(filename))
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new().part("file", part);
    let url = format!("{}/documents/extract", doc_service::base_url());

    let res = client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Document extract failed: {e}"))?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(parse_error_body(&text, status));
    }

    let parsed: ExtractResponse = res.json().await.map_err(|e| e.to_string())?;
    Ok(ExtractedDocument {
        text: parsed.text,
        page_count: parsed.page_count,
        char_count: parsed.char_count,
        format: if parsed.format.is_empty() {
            infer_format(filename)
        } else {
            parsed.format
        },
    })
}

pub async fn export_document_bytes(
    title: &str,
    body: &str,
    filename: &str,
    format: &str,
) -> Result<ExportedDocument, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/documents/export", doc_service::base_url());
    let res = client
        .post(&url)
        .json(&serde_json::json!({
            "title": title,
            "body": body,
            "filename": filename,
            "format": format,
        }))
        .send()
        .await
        .map_err(|e| format!("Document export failed: {e}"))?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(parse_error_body(&text, status));
    }

    let mime = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let disposition = res
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let export_name = parse_filename_from_disposition(disposition)
        .unwrap_or_else(|| default_export_name(filename, format));

    let bytes = res.bytes().await.map_err(|e| e.to_string())?.to_vec();
    Ok(ExportedDocument {
        bytes,
        filename: export_name,
        mime_type: mime,
    })
}

fn parse_error_body(text: &str, status: reqwest::StatusCode) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        if let Some(err) = v.get("error").and_then(|d| d.as_str()) {
            return err.to_string();
        }
        if let Some(detail) = v.get("detail").and_then(|d| d.as_str()) {
            return detail.to_string();
        }
    }
    if status == reqwest::StatusCode::PAYLOAD_TOO_LARGE {
        return "File too large (max 10 MB).".into();
    }
    format!("Service returned {status}: {text}")
}

fn parse_filename_from_disposition(header: &str) -> Option<String> {
    for part in header.split(';') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("filename=") {
            return Some(rest.trim_matches('"').to_string());
        }
    }
    None
}

pub fn infer_format(filename: &str) -> String {
    let lower = filename.to_lowercase();
    if lower.ends_with(".pdf") {
        return "pdf".into();
    }
    if lower.ends_with(".docx") {
        return "docx".into();
    }
    if lower.ends_with(".doc") {
        return "doc".into();
    }
    if lower.ends_with(".md") || lower.ends_with(".markdown") {
        return "md".into();
    }
    if lower.ends_with(".rtf") {
        return "rtf".into();
    }
    if lower.ends_with(".csv") {
        return "csv".into();
    }
    "txt".into()
}

fn default_export_name(original: &str, format: &str) -> String {
    let stem = original
        .rsplit_once('/')
        .map(|(_, s)| s)
        .unwrap_or(original)
        .rsplit_once('\\')
        .map(|(_, s)| s)
        .unwrap_or(original);
    let stem = stem.rsplit_once('.').map(|(s, _)| s).unwrap_or(stem);
    let ext = match format {
        "pdf" => "pdf",
        "docx" => "docx",
        "md" | "markdown" => "md",
        "rtf" => "rtf",
        "csv" => "csv",
        _ => "txt",
    };
    format!("{stem}.{ext}")
}

fn guess_mime(filename: &str) -> &'static str {
    let lower = filename.to_lowercase();
    if lower.ends_with(".pdf") {
        return "application/pdf";
    }
    if lower.ends_with(".docx") {
        return "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
    }
    if lower.ends_with(".txt") {
        return "text/plain";
    }
    if lower.ends_with(".md") {
        return "text/markdown";
    }
    if lower.ends_with(".rtf") {
        return "application/rtf";
    }
    if lower.ends_with(".csv") {
        return "text/csv";
    }
    "application/octet-stream"
}
