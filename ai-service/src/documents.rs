//! Uploaded documents and generated revisions.

use crate::pdf::{self, infer_format};
use crate::doc_service;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

pub const MAX_UPLOAD_BYTES: usize = 10 * 1024 * 1024;
const MAX_DOC_CHARS: usize = 48_000;
const MAX_LLM_DOC_CONTEXT: usize = 32_000;
const CHAT_PREVIEW_CHARS: usize = 6_000;

const ALLOWED_EXT: &[&str] = &[
    ".pdf", ".docx", ".txt", ".md", ".markdown", ".rtf", ".csv", ".log",
];

#[derive(Clone)]
pub struct DocumentRecord {
    pub id: String,
    pub session_id: String,
    pub filename: String,
    pub format: String,
    pub page_count: u32,
    pub extracted_text: String,
    pub current_text: String,
    pub latest_export_path: Option<PathBuf>,
    pub latest_export_filename: Option<String>,
    pub latest_mime_type: Option<String>,
    pub revision: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResponse {
    pub document_id: String,
    pub filename: String,
    pub format: String,
    pub page_count: u32,
    pub char_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentArtifact {
    pub document_id: String,
    pub filename: String,
    pub format: String,
    pub mime_type: String,
    pub revision: u32,
    pub download_url: String,
}

static DOCUMENTS: Mutex<Option<HashMap<String, DocumentRecord>>> = Mutex::new(None);

fn data_dir() -> PathBuf {
    let base = std::env::var("AI_DESK_DATA_DIR").unwrap_or_else(|_| "./.data".into());
    let dir = PathBuf::from(base).join("documents");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn store() -> std::sync::MutexGuard<'static, Option<HashMap<String, DocumentRecord>>> {
    let mut guard = DOCUMENTS.lock().expect("documents lock");
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

fn is_allowed_filename(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    ALLOWED_EXT.iter().any(|ext| lower.ends_with(ext))
}

pub fn truncate_for_context(text: &str) -> String {
    let t = text.trim();
    if t.len() <= MAX_LLM_DOC_CONTEXT {
        return t.to_string();
    }
    format!(
        "{}\n\n[Document truncated at {} characters — ask about a specific section if needed.]",
        &t[..MAX_LLM_DOC_CONTEXT],
        MAX_LLM_DOC_CONTEXT
    )
}

pub fn document_context_for_llm(doc: &DocumentRecord) -> String {
    format!(
        "Attached document: \"{}\" (format {}, {} pages, revision {}).\n\
         Answer ONLY from this file — do not use the web or outside knowledge unless the employee \
         explicitly asks to compare with external information.\n\n\
         --- Document text ---\n{}\n--- End document ---",
        doc.filename,
        doc.format,
        doc.page_count,
        doc.revision,
        truncate_for_context(&doc.current_text)
    )
}

pub fn chat_preview(text: &str) -> String {
    let t = text.trim();
    if t.len() <= CHAT_PREVIEW_CHARS {
        return t.to_string();
    }
    format!("{}…", &t[..CHAT_PREVIEW_CHARS])
}

pub async fn upload_document(
    session_id: &str,
    filename: &str,
    bytes: &[u8],
) -> Result<UploadResponse, String> {
    if !is_allowed_filename(filename) {
        return Err(format!(
            "Unsupported file type. Use: {}",
            ALLOWED_EXT.join(", ")
        ));
    }
    if bytes.is_empty() {
        return Err("Empty file.".into());
    }
    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err("File too large (max 10 MB).".into());
    }

    if !doc_service::is_available().await {
        return Err(
            "Document upload needs the document service on port 8092. \
             Start it with start.cmd (or start-docs.cmd)."
                .into(),
        );
    }

    let extracted = pdf::extract_document_bytes(bytes, filename).await?;
    let mut text = extracted.text;
    if text.len() > MAX_DOC_CHARS {
        text = format!(
            "{}\n\n[Truncated at {} characters for processing.]",
            &text[..MAX_DOC_CHARS],
            MAX_DOC_CHARS
        );
    }

    let id = Uuid::new_v4().to_string();
    let record = DocumentRecord {
        id: id.clone(),
        session_id: session_id.to_string(),
        filename: filename.to_string(),
        format: extracted.format.clone(),
        page_count: extracted.page_count,
        extracted_text: text.clone(),
        current_text: text.clone(),
        latest_export_path: None,
        latest_export_filename: None,
        latest_mime_type: None,
        revision: 0,
    };

    {
        let mut guard = store();
        guard.as_mut().expect("documents map").insert(id.clone(), record);
    }

    Ok(UploadResponse {
        document_id: id,
        filename: filename.to_string(),
        format: extracted.format,
        page_count: extracted.page_count,
        char_count: extracted.char_count,
    })
}

pub fn get(document_id: &str) -> Option<DocumentRecord> {
    let guard = store();
    guard
        .as_ref()
        .and_then(|m| m.get(document_id).cloned())
}

pub fn get_for_session(document_id: &str, session_id: &str) -> Option<DocumentRecord> {
    get(document_id).filter(|d| d.session_id == session_id)
}

pub fn is_document_edit_request(message: &str) -> bool {
    let lower = message.to_lowercase();
    let verbs = [
        "edit",
        "improve",
        "rewrite",
        "revise",
        "update",
        "fix",
        "polish",
        "reformat",
        "redraft",
        "regenerate",
        "create a new version",
        "export",
        "download",
        "save as",
        "give me the file",
        "send me the",
    ];
    verbs.iter().any(|v| lower.contains(v))
        || lower.contains("the document")
        || lower.contains("this document")
        || lower.contains("attached file")
}

pub fn export_format_for_request(message: &str, doc_format: &str) -> String {
    let lower = message.to_lowercase();
    if lower.contains("pdf") {
        return "pdf".into();
    }
    if lower.contains("word") || lower.contains("docx") {
        return "docx".into();
    }
    if lower.contains("markdown") || lower.contains(" as md") {
        return "md".into();
    }
    if lower.contains("plain text") || lower.contains(" as txt") {
        return "txt".into();
    }
    infer_format(&format!("file.{doc_format}"))
}

pub async fn apply_edit(
    document_id: &str,
    session_id: &str,
    instruction: &str,
    edited_body: &str,
) -> Result<(DocumentArtifact, String), String> {
    let (title, export_fmt) = {
        let doc = get_for_session(document_id, session_id)
            .ok_or_else(|| "Document not found for this session.".to_string())?;
        let title = doc
            .filename
            .rsplit_once('.')
            .map(|(s, _)| s.to_string())
            .unwrap_or_else(|| doc.filename.clone());
        let fmt = export_format_for_request(instruction, &doc.format);
        (title, fmt)
    };

    let body = edited_body.trim();
    if body.is_empty() {
        return Err("Model returned empty document text.".into());
    }

    let preview = chat_preview(body);
    let revision;
    let filename;
    let mime_type;
    let path;

    {
        let mut guard = store();
        let map = guard.as_mut().expect("documents map");
        let doc = map
            .get_mut(document_id)
            .ok_or_else(|| "Document not found.".to_string())?;
        if doc.session_id != session_id {
            return Err("Document not found for this session.".into());
        }
        doc.revision += 1;
        revision = doc.revision;
        doc.current_text = body.to_string();
    }

    let exported = pdf::export_document_bytes(
        &title,
        body,
        &format!("{title}-rev{revision}.{export_fmt}"),
        &export_fmt,
    )
    .await?;

    filename = exported.filename.clone();
    mime_type = exported.mime_type.clone();
    path = data_dir().join(format!("{document_id}-rev{revision}-{}", exported.filename));

    std::fs::write(&path, &exported.bytes).map_err(|e| e.to_string())?;

    {
        let mut guard = store();
        if let Some(doc) = guard.as_mut().and_then(|m| m.get_mut(document_id)) {
            doc.latest_export_path = Some(path);
            doc.latest_export_filename = Some(filename.clone());
            doc.latest_mime_type = Some(mime_type.clone());
        }
    }

    let artifact = DocumentArtifact {
        document_id: document_id.to_string(),
        filename,
        format: export_fmt,
        mime_type,
        revision,
        download_url: format!("/api/documents/{document_id}/download"),
    };

    Ok((artifact, preview))
}

pub fn read_latest_export(document_id: &str, session_id: &str) -> Result<(Vec<u8>, String, String), String> {
    let doc = get_for_session(document_id, session_id)
        .ok_or_else(|| "Document not found.".to_string())?;
    let path = doc
        .latest_export_path
        .as_ref()
        .ok_or_else(|| "No export yet — ask to edit or improve the document first.".to_string())?;
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let name = doc
        .latest_export_filename
        .clone()
        .unwrap_or_else(|| doc.filename.clone());
    let mime = doc
        .latest_mime_type
        .clone()
        .unwrap_or_else(|| "application/octet-stream".to_string());
    Ok((bytes, name, mime))
}

pub fn document_context(doc: &DocumentRecord) -> String {
    document_context_for_llm(doc)
}
