# Configuration

Configuration is loaded from **`.env`** (project root) when services start via `desk.ps1`. Spring also reads `src/main/resources/application.properties`.

---

## `.env` — primary secrets and RAG

Copy from `.env.example`:

```env
QDRANT_URL=https://your-cluster.region.gcp.cloud.qdrant.io
QDRANT_API_KEY=your-api-key
QDRANT_COLLECTION=ncr-desk-kb
```

| Variable | Required | Description |
|----------|----------|-------------|
| `QDRANT_URL` | For RAG | Qdrant cluster URL (no `/collections` path) |
| `QDRANT_API_KEY` | For RAG | API key from Qdrant Cloud |
| `QDRANT_COLLECTION` | No | Collection name (default `ncr-desk-kb`) |

If `QDRANT_URL` or `QDRANT_API_KEY` is missing, RAG is disabled; **local KB search still works**.

---

## Rust API (`8090`)

| Variable | Default | Description |
|----------|---------|-------------|
| `AI_DESK_BIND` | `127.0.0.1:8090` | Listen address |
| `AI_DESK_USE_LLM` | `auto` | `auto`, `on`, `off` — when to call Qwen/Ollama |
| `AI_DESK_DATA_DIR` | `./.data` | Document storage root |
| `QWEN_INFERENCE_URL` | `http://127.0.0.1:8092` | Python service base URL |
| `AI_DESK_FAST_KB_SCORE` | (see `chat.rs`) | Min KB score to skip LLM |
| `AI_DESK_ALWAYS_LLM` | off | If set, always call LLM for KB answers |
| `QWEN_ASSISTANT_MAX_TOKENS` | `220` | Max tokens for writing tasks |
| `QWEN_DOCUMENT_MAX_TOKENS` | `380` | Max tokens for document edit |
| `OLLAMA_BASE_URL` | `http://127.0.0.1:11434` | Fallback LLM |
| `OLLAMA_MODEL` | `llama3.2` | Ollama model name |
| `OLLAMA_MAX_TOKENS` | `96` | Ollama generation limit |
| `EMBED_SERVICE_URL` | (derived from Qwen URL) | Override embed endpoint |

---

## Python Qwen (`8092`)

Set by `desk.ps1` when starting Qwen:

| Variable | Default | Description |
|----------|---------|-------------|
| `QWEN_MODEL_DIR` | `./qwen-model` | Hugging Face model directory |
| `QWEN_BIND_HOST` | `127.0.0.1` | Bind host |
| `QWEN_BIND_PORT` | `8092` | Bind port |
| `QWEN_MAX_NEW_TOKENS` | `96` | Default generation length |
| `QWEN_MAX_INPUT_TOKENS` | `1024` | Input truncation |
| `QWEN_THREADS` | `4` | torch CPU threads |

---

## Spring (`8080`)

`src/main/resources/application.properties`:

```properties
server.port=8080
spring.servlet.multipart.max-file-size=10MB
spring.servlet.multipart.max-request-size=10MB
ai.desk.backend-url=http://127.0.0.1:8090
```

| Property | Description |
|----------|-------------|
| `ai.desk.backend-url` | Rust API base URL for `AiDeskClient` |

---

## Upload limit (10 MB)

Enforced consistently at:

- Spring multipart settings  
- Rust `documents::MAX_UPLOAD_BYTES` and Actix payload config  
- Python `MAX_UPLOAD_BYTES` in `server.py`  
- Browser check in `app.js`  

---

## Tuning scenarios

| Goal | Setting |
|------|---------|
| Fastest directory-only desk | `AI_DESK_USE_LLM=off` |
| Always paraphrase with AI | `AI_DESK_ALWAYS_LLM=1` |
| Shorter/faster Qwen replies | Lower `QWEN_MAX_NEW_TOKENS` |
| Disable RAG | Remove Qdrant vars from `.env` |
| Use Ollama instead of Qwen | Run Ollama; ensure `ollama_available` in health |

See also [TRAINING.md](TRAINING.md) for KB and prompt changes.
