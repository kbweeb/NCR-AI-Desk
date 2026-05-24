# Setup — recreate the NCR AI Desk from scratch

This guide lets a new developer or evaluator build and run the full stack on **Windows** without prior knowledge of the repo.

---

## 1. What you are building

Three processes that must run together:

| # | Service | Port | Started by |
|---|---------|------|------------|
| 1 | Qwen inference (Python) | 8092 | `start-qwen.cmd` or `start.cmd` |
| 2 | Rust API | 8090 | `start-api.cmd` or `start.cmd` |
| 3 | Spring Boot UI | 8080 | `start-web.cmd` or `start.cmd` |

The browser only talks to **8080**. Spring forwards API calls to **8090**. Rust calls **8092** for AI and document parsing.

---

## 2. Prerequisites

| Requirement | Minimum | Notes |
|-------------|---------|--------|
| OS | Windows 10/11 | `desk.ps1` is PowerShell; ports 8080, 8090, 8092 must be free |
| Python | 3.10+ | On PATH as `python` |
| Disk | ~5 GB free | Model ~1 GB + venv + Maven/JDK caches in `.tools/` |
| RAM | 8 GB+ | Qwen 0.5B + OS; more is better |
| Network | First run | Downloads model, JDK, Maven, or Rust if missing |

**Optional (auto-installed by `desk.ps1` if missing):**

- Rust / `cargo`
- Java 17 JDK
- Apache Maven

**Optional (for RAG):**

- Qdrant Cloud account — URL + API key in `.env`

---

## 3. Get the code

```powershell
git clone https://github.com/kbweeb/NCR-AIDesk.git
cd NCR-AIDesk
```

---

## 4. Configure secrets

```powershell
copy .env.example .env
```

Edit `.env`:

```env
QDRANT_URL=https://YOUR-CLUSTER.region.gcp.cloud.qdrant.io
QDRANT_API_KEY=your-api-key
QDRANT_COLLECTION=ncr-desk-kb
```

RAG is optional: without Qdrant, **directory search still works** via the built-in KB in `ai-service/src/kb.rs`.

---

## 5. Download the Qwen model (one time)

```powershell
.\download-model.cmd
```

This downloads **Qwen/Qwen2.5-0.5B-Instruct** into `qwen-model/` (requires `pip install huggingface_hub` or the `hf` CLI).

Verify:

```powershell
Test-Path .\qwen-model\config.json
# Should be True
```

---

## 6. Start the stack

```powershell
.\start.cmd
```

Three PowerShell windows should open:

1. **Qwen** — loads model; wait until you see “Qwen ready” / server listening on 8092  
2. **Rust API** — “listening on http://127.0.0.1:8090”  
3. **Spring** — “Started …” on port 8080  

Open **http://127.0.0.1:8080/** and check the status pill:

- **Online** — API + Qwen ready  
- **Directory mode** — API up, Qwen still loading or off (KB-only answers work)  
- **Offline** — Spring or Rust not reachable  

---

## 7. Verify each layer (health checks)

Run in a fourth terminal:

```powershell
# Spring (proxies Rust health)
Invoke-RestMethod http://127.0.0.1:8080/api/health | ConvertTo-Json -Depth 5

# Rust directly
Invoke-RestMethod http://127.0.0.1:8090/health | ConvertTo-Json -Depth 5

# Qwen directly
Invoke-RestMethod http://127.0.0.1:8092/health | ConvertTo-Json -Depth 5
```

Expected:

- Spring: `backendReachable: true` when Rust is up  
- Rust: `status: "ok"`  
- Qwen: `ready: true` after model load (may be `false` for 1–2 minutes on first start)

---

## 8. Smoke test in the UI

1. Type: `Who do I contact for IT support?` — should return IT Service Desk number from KB (fast).  
2. Type: `hi` — greeting, no error.  
3. Attach a small `.txt` or `.docx` under 10 MB — should confirm file attached.  
4. Ask: `Summarize this document in three bullets` — needs Qwen **Online**.

Full evaluation checklist: [EVALUATION.md](EVALUATION.md).

---

## 9. Stop and clean up

```powershell
.\stop.cmd
```

To reset document uploads: delete `.data/documents/` (created at runtime).

To reinstall Python deps:

```powershell
Remove-Item -Recurse -Force .tools\qwen-venv
.\start-qwen.cmd
```

---

## 10. Manual start (without `start.cmd`)

If you prefer separate terminals:

```powershell
# Terminal 1
.\start-qwen.cmd

# Terminal 2 (after Qwen begins loading)
.\start-api.cmd

# Terminal 3
.\start-web.cmd
```

Or PowerShell directly:

```powershell
.\desk.ps1 -Action qwen
.\desk.ps1 -Action api
.\desk.ps1 -Action web
```

---

## 11. Common setup failures

| Symptom | Cause | Fix |
|---------|--------|-----|
| Status stuck “Connecting” | Spring or Rust not running | Run `start.cmd`; check 8090/8080 |
| “Directory mode” forever | Qwen not ready | Wait for model load; check 8092 `/health` `ready: true` |
| Upload “Internal Server Error” | Spring 1 MB limit (old build) or Qwen down | Restart Spring; ensure 8092; file ≤ 10 MB |
| `pip install` fails | No Python / no network | Install Python 3.10+; retry `start-qwen.cmd` |
| `cargo run` fails | Rust not installed | Let `desk.ps1` install rustup, or install from rustup.rs |
| RAG errors in Rust console | Bad Qdrant URL/key | Fix `.env`; desk still works with local KB |
| Port already in use | Previous run | `.\stop.cmd` |

---

## 12. Recreate on another machine (checklist)

- [ ] Clone repo  
- [ ] `copy .env.example .env` and fill Qdrant (optional)  
- [ ] `download-model.cmd`  
- [ ] `start.cmd`  
- [ ] Health checks on 8080, 8090, 8092  
- [ ] UI smoke tests from [EVALUATION.md](EVALUATION.md)  
- [ ] `cargo test` in `ai-service/`  

You do **not** need to copy `qwen-model/`, `.tools/`, `.data/`, or `target/` from another PC — regenerate them locally.
