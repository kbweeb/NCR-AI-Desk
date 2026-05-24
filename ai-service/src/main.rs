mod chat;
mod documents;
mod embeddings;
mod kb;
mod llm;
mod nlp;
mod pdf;
mod qdrant_rag;
mod doc_service;
mod perplexity;
mod session;

use documents::MAX_UPLOAD_BYTES;
use actix_multipart::Multipart;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chat::AskContext;
use futures_util::{future, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use session::ChatTurn;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AskRequest {
    message: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    history: Vec<ChatTurn>,
    #[serde(default)]
    document_id: Option<String>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
    llm: LlmHealth,
    rag: RagHealth,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LlmHealth {
    mode: String,
    live_available: bool,
    live_model: String,
    document_service_available: bool,
    document_service_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RagHealth {
    configured: bool,
    ready: bool,
    collection: String,
    qdrant_url: Option<String>,
}

async fn health() -> impl Responder {
    let (live_available, doc_available, rag_ready) = future::join3(
        llm::live_llm_available(),
        doc_service::is_available(),
        qdrant_rag::is_ready(),
    )
    .await;

    HttpResponse::Ok().json(HealthResponse {
        status: "ok".into(),
        service: "ncr-tech-solutions-desk".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        llm: LlmHealth {
            mode: llm::use_llm_mode(),
            live_available,
            live_model: perplexity::model(),
            document_service_available: doc_available,
            document_service_url: doc_service::base_url(),
        },
        rag: RagHealth {
            configured: qdrant_rag::is_configured(),
            ready: rag_ready,
            collection: qdrant_rag::collection_name(),
            qdrant_url: qdrant_rag::qdrant_url(),
        },
    })
}

async fn ask(payload: web::Json<AskRequest>) -> impl Responder {
    let req = payload.into_inner();
    let response = chat::answer(AskContext {
        message: &req.message,
        session_id: req.session_id.as_deref(),
        history: &req.history,
        document_id: req.document_id.as_deref(),
    })
    .await;
    HttpResponse::Ok().json(response)
}

async fn upload_document(mut payload: Multipart) -> impl Responder {
    let mut session_id = String::new();
    let mut file_name = String::new();
    let mut file_bytes: Option<Vec<u8>> = None;

    while let Ok(Some(mut field)) = payload.try_next().await {
        let name = field
            .content_disposition()
            .and_then(|d| d.get_name().map(|s| s.to_string()))
            .unwrap_or_default();

        let mut bytes = Vec::new();
        while let Some(chunk) = field.next().await {
            match chunk {
                Ok(data) => {
                    if bytes.len() + data.len() > MAX_UPLOAD_BYTES {
                        return HttpResponse::PayloadTooLarge().json(serde_json::json!({
                            "error": "File too large (max 10 MB)."
                        }));
                    }
                    bytes.extend_from_slice(&data);
                }
                Err(e) => {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": format!("Upload read error: {e}")
                    }));
                }
            }
        }

        match name.as_str() {
            "sessionId" => session_id = String::from_utf8_lossy(&bytes).trim().to_string(),
            "file" => {
                file_name = field
                    .content_disposition()
                    .and_then(|d| d.get_filename().map(|s| s.to_string()))
                    .unwrap_or_else(|| "document.pdf".to_string());
                file_bytes = Some(bytes);
            }
            _ => {}
        }
    }

    if session_id.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "sessionId required"
        }));
    }

    let Some(bytes) = file_bytes else {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Document file required"
        }));
    };

    match documents::upload_document(&session_id, &file_name, &bytes).await {
        Ok(resp) => HttpResponse::Ok().json(resp),
        Err(err) => HttpResponse::BadRequest().json(serde_json::json!({ "error": err })),
    }
}

async fn download_document(path: web::Path<String>, query: web::Query<DownloadQuery>) -> impl Responder {
    let document_id = path.into_inner();
    let session_id = query.session_id.as_deref().unwrap_or("");
    match documents::read_latest_export(&document_id, session_id) {
        Ok((bytes, filename, mime)) => HttpResponse::Ok()
            .content_type(mime)
            .insert_header((
                "Content-Disposition",
                format!("attachment; filename=\"{filename}\""),
            ))
            .body(bytes),
        Err(err) => HttpResponse::NotFound().json(serde_json::json!({ "error": err })),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadQuery {
    session_id: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let bind = std::env::var("AI_DESK_BIND").unwrap_or_else(|_| "127.0.0.1:8090".to_string());

    let live_on = llm::live_llm_available().await;
    let doc_on = doc_service::is_available().await;

    println!("NCR Tech Solutions AI Desk API listening on http://{bind}/");
    if live_on {
        println!("  LLM: Perplexity Sonar ({})", perplexity::model());
    } else {
        println!("  LLM: offline — set PERPLEXITY_API_KEY in .env for live answers");
    }
    if doc_on {
        println!("  Documents: {}", doc_service::base_url());
    } else {
        println!("  Documents: offline — start document service on port 8092");
    }

    if qdrant_rag::is_configured() {
        println!(
            "  RAG: Qdrant Cloud ({})",
            qdrant_rag::qdrant_url().unwrap_or_default()
        );
        qdrant_rag::sync_if_ready().await;
        tokio::spawn(async {
            for attempt in 1..=18 {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                if !qdrant_rag::is_configured() {
                    break;
                }
                if qdrant_rag::is_ready().await {
                    if let Ok(n) = qdrant_rag::index_kb().await {
                        println!("  RAG: indexed {n} entries (attempt {attempt})");
                    }
                    break;
                }
            }
        });
    } else {
        println!("  RAG: set QDRANT_URL + QDRANT_API_KEY in .env for vector search");
    }

    println!("  Documents: PDF/Word upload, Q&A, edit, export (document service :8092 + Perplexity)");
    println!("  Frontend: Spring Boot http://127.0.0.1:8080/");

    let payload_limit = MAX_UPLOAD_BYTES + (512 * 1024);
    HttpServer::new(move || {
        App::new()
            .app_data(web::PayloadConfig::new(payload_limit))
            .route("/health", web::get().to(health))
            .route("/api/ask", web::post().to(ask))
            .route("/api/documents/upload", web::post().to(upload_document))
            .route(
                "/api/documents/{id}/download",
                web::get().to(download_document),
            )
    })
    .bind(bind)?
    .run()
    .await
}
