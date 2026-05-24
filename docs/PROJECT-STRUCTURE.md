# Project structure — file reference

Every **source** path in the repo and why it exists. Ignored paths: `target/`, `qwen-model/`, `.tools/`, `.data/`, `__pycache__/`, `.idea/`, `.vscode/` (unless you use them locally).

---

## Repository root

| File / folder | Purpose |
|---------------|---------|
| **`desk.ps1`** | Unified launcher: start/stop services, download model, ensure JDK/Maven/Rust, load `.env` |
| **`start.cmd`** | Runs `desk.ps1 -Action all` |
| **`run.cmd`** | Alias for `start.cmd` |
| **`stop.cmd`** | Runs `desk.ps1 -Action stop` |
| **`download-model.cmd`** | Runs `desk.ps1 -Action download-model` |
| **`start-qwen.cmd`** | Qwen only |
| **`start-api.cmd`** | Rust API only |
| **`start-web.cmd`** | Spring UI only |
| **`pom.xml`** | Maven project for Spring Boot frontend |
| **`README.md`** | Project overview and doc index |
| **`.env.example`** | Template for Qdrant and optional overrides |
| **`.env`** | Local secrets (gitignored) — **you create this** |
| **`docs/`** | Full documentation set |

---

## `src/main/java/` — Spring Boot (port 8080)

| Path | Purpose |
|------|---------|
| `com/ncr/desk/AiDeskFrontendApplication.java` | Spring Boot entry point |
| `com/ncr/desk/config/DeskProperties.java` | `ai.desk.backend-url` property |
| `com/ncr/desk/config/DeskConfiguration.java` | Registers `AiDeskClient` bean |
| `com/ncr/desk/api/DeskApiController.java` | REST: health, ask, upload, download |
| `com/ncr/desk/api/AiDeskClient.java` | HTTP client to Rust (short timeout health, 120s ask/upload) |
| `com/ncr/desk/api/DeskExceptionHandler.java` | Upload size / multipart errors → JSON `ApiError` |
| `com/ncr/desk/api/dto/AskRequest.java` | Incoming chat JSON |
| `com/ncr/desk/api/dto/AskResponse.java` | Outgoing chat JSON |
| `com/ncr/desk/api/dto/ChatTurn.java` | `{ role, content }` history item |
| `com/ncr/desk/api/dto/BackendHealth.java` | Rust `/health` deserialization |
| `com/ncr/desk/api/dto/FrontendHealth.java` | UI health wrapper + `aiAvailable` |
| `com/ncr/desk/api/dto/DocumentUploadResponse.java` | Upload success payload |
| `com/ncr/desk/api/dto/DocumentArtifact.java` | Download metadata after edit |
| `com/ncr/desk/api/dto/DocumentDownload.java` | Internal: bytes + MIME for download |
| `com/ncr/desk/api/dto/ApiError.java` | `{ "error": "..." }` |

### `src/main/resources/`

| Path | Purpose |
|------|---------|
| `application.properties` | Port 8080, **10 MB** multipart, backend URL `http://127.0.0.1:8090` |
| `static/index.html` | Chat UI layout (sidebar, bubbles, composer) |
| `static/styles.css` | ChatGPT-style dark theme, message bubbles |
| `static/app.js` | Chat logic, health polling, upload, sessionStorage history |

---

## `ai-service/` — Rust API (port 8090)

| Path | Purpose |
|------|---------|
| `Cargo.toml` | Crate deps: actix-web, reqwest, serde, multipart, uuid |
| `Cargo.lock` | Locked versions |

### `ai-service/src/`

| Module | Purpose |
|--------|---------|
| **`main.rs`** | HTTP server, routes, multipart upload handler, 10 MB payload limit |
| **`chat.rs`** | **Core orchestration** — all answer paths, document mode, follow-ups |
| **`kb.rs`** | **Knowledge base** — static `KbEntry` list (phones, HR, IT, procedures) |
| **`nlp.rs`** | Tokenize, intent rules, off-topic/greeting/assistant detection |
| **`session.rs`** | In-memory per-session history (24 turns max) |
| **`llm.rs`** | System prompts; Qwen/Ollama calls; when to use LLM |
| **`qwen.rs`** | HTTP client to Python `:8092` (`/health`, `/chat`) |
| **`embeddings.rs`** | HTTP client to Python `/embed` for RAG vectors |
| **`qdrant_rag.rs`** | Qdrant index + search; retries; URL normalization for cloud |
| **`documents.rs`** | Document IDs, storage, edit/export orchestration, 10 MB limit |
| **`pdf.rs`** | HTTP bridge to Python `/documents/extract` and `/documents/export` |

---

## `qwen-service/` — Python inference (port 8092)

| Path | Purpose |
|------|---------|
| **`server.py`** | FastAPI: `/health`, `/chat`, `/embed`, `/documents/extract`, `/documents/export`, legacy `/pdf/extract` |
| **`document_tools.py`** | Extract PDF/DOCX/txt/md/rtf/csv; export PDF/DOCX/text |
| **`requirements.txt`** | torch, transformers, fastembed, pypdf, python-docx, fpdf2, fastapi, uvicorn |

---

## `docs/`

| File | Purpose |
|------|---------|
| `SETUP.md` | Recreate system from scratch |
| `ARCHITECTURE.md` | Flows and design |
| `PROJECT-STRUCTURE.md` | This file |
| `API.md` | HTTP API reference |
| `CONFIGURATION.md` | Environment variables |
| `EVALUATION.md` | Testing and evaluation guide |
| `TRAINING.md` | Customize KB and behavior |

---

## Dependency graph (who calls whom)

```text
app.js
  → DeskApiController (Spring)
    → AiDeskClient
      → main.rs (Rust)
        → chat.rs
          → kb.rs | qdrant_rag.rs | nlp.rs | session.rs | llm.rs
          → documents.rs → pdf.rs
        → qwen.rs / embeddings.rs
          → server.py (Python)
            → document_tools.py
```

---

## What to edit for common changes

| Goal | File(s) |
|------|---------|
| Add FAQ / phone number | `ai-service/src/kb.rs` |
| Change AI tone / rules | `ai-service/src/llm.rs` |
| Change routing (when to use AI) | `ai-service/src/chat.rs`, `nlp.rs` |
| UI look | `src/main/resources/static/*` |
| Upload size | `documents.rs`, `server.py`, `application.properties`, `app.js` |
| New file format | `document_tools.py`, `documents.rs` ALLOWED_EXT |
| Secrets / RAG | `.env` |
