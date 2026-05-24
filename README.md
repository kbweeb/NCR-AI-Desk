# NCR Tech Solutions — AI Desk

Internal workplace assistant for NCR employees: **directory and policy answers**, **document upload and editing** (PDF, Word, text), and an **AI copilot** with **live web data** (weather, news, current events) via **Perplexity Sonar**. Optional **Qdrant Cloud RAG** improves semantic search over the knowledge base.

**Repository:** [github.com/kbweeb/NCR-AIDesk](https://github.com/kbweeb/NCR-AIDesk)

---

## What this system does

| Capability | How it works |
|------------|----------------|
| IT phone, HR, rooms, complaints | Built-in knowledge base in Rust (`kb.rs`), instant or RAG-enhanced |
| “Help me write an email” | Perplexity Sonar (live web) |
| “Weather in London today” / latest news | Perplexity Sonar (searches the web) |
| Upload & summarize a Word/PDF file | Extract text in Python → store in Rust → chat with document context |
| Edit document & download | LLM rewrite → export PDF/DOCX/txt via Python |
| Off-topic / casual chat | Short redirect back to work scope |

---

## Architecture (three services)

| Layer | Port | Technology | Role |
|-------|------|------------|------|
| **Web UI** | 8080 | Spring Boot + static HTML/JS | Browser UI; proxies `/api/*` to Rust |
| **API** | 8090 | Rust (Actix Web) | Routing, KB, sessions, documents, RAG, LLM orchestration |
| **Inference** | 8092 | Python (FastAPI + Qwen) | Chat, embeddings, file extract/export |

```text
Browser → Spring :8080 → Rust :8090 → Python :8092 (chat / documents / embed)
                              ↓
                         Qdrant Cloud (optional RAG)
```

**Full detail:** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

---

## Quick start (Windows)

**Prerequisites:** Windows 10/11, Python 3.10+, internet for first-time tool/model download. Rust/Java/Maven can be auto-installed into `.tools/` by `desk.ps1`.

### 1. Clone and configure

```powershell
git clone https://github.com/kbweeb/NCR-AIDesk.git
cd NCR-AIDesk
copy .env.example .env
# Edit .env — set PERPLEXITY_API_KEY (required for chat)
# Optional: QDRANT_URL and QDRANT_API_KEY for directory RAG
```

Get a Perplexity API key: https://www.perplexity.ai/settings/api

You can delete the old local model folder if it exists: `Remove-Item -Recurse -Force qwen-model`

### 2. Start everything

```powershell
.\start.cmd
```

Open **http://127.0.0.1:8080/** — wait until status shows **Online** or **Directory mode**.

### 4. Stop

```powershell
.\stop.cmd
```

| Command | Purpose |
|---------|---------|
| `start.cmd` / `run.cmd` | Start document service + Rust API + web UI |
| `stop.cmd` | Free ports 8092, 8090, 8080 |
| `start-docs.cmd` | Python document service only (port 8092) |
| `start-api.cmd` | Rust API only |
| `start-web.cmd` | Spring UI only |

Launcher implementation: **`desk.ps1`** (single PowerShell entry point).

---

## Documentation index

| Document | Audience | Contents |
|----------|----------|----------|
| [docs/SETUP.md](docs/SETUP.md) | Developers / evaluators | **Recreate the system from scratch** — prerequisites, install, verify |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Technical | Data flows, modules, ports, design decisions |
| [docs/PROJECT-STRUCTURE.md](docs/PROJECT-STRUCTURE.md) | Technical | **Every important file** and what it contains |
| [docs/API.md](docs/API.md) | Integrators | HTTP endpoints (Spring + Rust + Python) |
| [docs/CONFIGURATION.md](docs/CONFIGURATION.md) | Operators | Environment variables and tuning |
| [docs/EVALUATION.md](docs/EVALUATION.md) | Reviewers | **How to test and evaluate** the desk objectively |
| [docs/TRAINING.md](docs/TRAINING.md) | Content owners | Customize KB, prompts, RAG (no ML training required) |

---

## Tests

```powershell
cd ai-service
cargo test
```

Manual and integration checks: [docs/EVALUATION.md](docs/EVALUATION.md).

---

## License and use

Internal NCR Tech Solutions demonstration / employee desk. Do not commit `.env`, `qwen-model/`, `.tools/`, `.data/`, or `target/` — they are local runtime artifacts.
