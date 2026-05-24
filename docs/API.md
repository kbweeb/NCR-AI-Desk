# HTTP API reference

The browser uses **Spring** on port **8080**. Spring proxies to **Rust** on **8090**. Rust calls **Python** on **8092** internally (not exposed to the browser).

Base URLs:

- UI + Spring API: `http://127.0.0.1:8080`
- Rust (direct): `http://127.0.0.1:8090`
- Qwen (direct): `http://127.0.0.1:8092`

---

## Spring — browser-facing (`/api/*`)

### `GET /api/health`

Aggregated health for the UI status pill.

**Response (example):**

```json
{
  "status": "ok",
  "frontend": "spring-boot",
  "backendReachable": true,
  "aiAvailable": true,
  "backend": {
    "status": "ok",
    "service": "ncr-tech-solutions-desk",
    "version": "0.1.0",
    "llm": {
      "mode": "auto",
      "qwenAvailable": true,
      "qwenUrl": "http://127.0.0.1:8092",
      "ollamaAvailable": false,
      "ollamaModel": "llama3.2",
      "ollamaBaseUrl": "http://127.0.0.1:11434"
    }
  }
}
```

| Field | Meaning |
|-------|---------|
| `backendReachable` | Rust API responded |
| `aiAvailable` | Qwen or Ollama passed health check |

---

### `POST /api/ask`

Main chat endpoint.

**Request:**

```json
{
  "message": "Who do I contact for IT support?",
  "sessionId": "uuid-from-browser",
  "history": [
    { "role": "user", "content": "hi" },
    { "role": "assistant", "content": "Welcome..." }
  ],
  "documentId": null
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `message` | Yes | User text |
| `sessionId` | Recommended | Ties server session + documents |
| `history` | No | Prior turns from browser (max 24) |
| `documentId` | No | Active uploaded document for Q&A/edit |

**Response (example):**

```json
{
  "reply": "For IT support, contact **NCR IT Service Desk** at +1 (800) 225-5627.",
  "intent": "contact_lookup",
  "confidence": 0.9,
  "engine": "local",
  "sources": [{ "id": "phone-it", "title": "...", "category": "it", "score": 0.95 }],
  "suggestedFollowUps": [],
  "documentArtifact": null,
  "activeDocumentId": null,
  "documentEditPreview": null
}
```

| Field | Description |
|-------|-------------|
| `engine` | `local`, `rag`, `qwen`, `ollama` |
| `documentEditPreview` | Excerpt of edited document text (after edit) |
| `documentArtifact` | Present when a downloadable export exists |

**Errors:** `502` if Rust unreachable; timeout if Qwen slow (Spring read timeout 120s).

---

### `POST /api/documents/upload`

**Content-Type:** `multipart/form-data`

| Part | Required | Description |
|------|----------|-------------|
| `sessionId` | Yes | Browser session UUID |
| `file` | Yes | Document binary |

**Limits:** 10 MB (Spring, Rust, Python).

**Success response:**

```json
{
  "documentId": "uuid",
  "filename": "report.docx",
  "format": "docx",
  "pageCount": 3,
  "charCount": 4521
}
```

**Errors:**

```json
{ "error": "File too large (max 10 MB)." }
```

HTTP `413` or `400` with `ApiError` body.

---

### `GET /api/documents/{id}/download?sessionId={uuid}`

Returns file bytes with correct `Content-Type` and `Content-Disposition` filename.

**Errors:** `404` if no export or wrong session; `502` if Rust down.

---

## Rust — direct (`8090`)

Same paths as Spring proxies (except no `/api` prefix on health):

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Service + LLM + RAG status |
| `POST` | `/api/ask` | Same body as Spring |
| `POST` | `/api/documents/upload` | Multipart upload |
| `GET` | `/api/documents/{id}/download` | Query `sessionId` required |

---

## Python — inference (`8092`)

Used by Rust, not the browser.

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | `{ "status", "ready", "modelDir", "embeddingsReady" }` |
| `POST` | `/chat` | Body: `{ "system", "user", "max_new_tokens"? }` → `{ "reply" }` |
| `POST` | `/embed` | Body: `{ "texts": ["..."] }` → `{ "vectors": [[...]] }` |
| `POST` | `/documents/extract` | Multipart `file` → text + metadata |
| `POST` | `/documents/export` | JSON: `title`, `body`, `filename`, `format` → file bytes |
| `POST` | `/pdf/extract` | Legacy PDF-only extract |

---

## Example: direct Rust ask (PowerShell)

```powershell
$body = @{
  message = "Who do I contact for IT support?"
  sessionId = [guid]::NewGuid().ToString()
  history = @()
  documentId = $null
} | ConvertTo-Json

Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8090/api/ask" `
  -ContentType "application/json" -Body $body
```

---

## Example: upload (curl)

```bash
curl -X POST "http://127.0.0.1:8080/api/documents/upload" \
  -F "sessionId=test-session-1" \
  -F "file=@report.docx"
```
