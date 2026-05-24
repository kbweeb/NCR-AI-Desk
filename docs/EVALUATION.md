# Evaluation guide

Use this document to **test**, **demo**, and **grade** the NCR AI Desk objectively. Suitable for technical reviewers, course evaluators, or internal stakeholders.

---

## 1. Evaluation criteria

| Criterion | Weight | What “good” looks like |
|-----------|--------|-------------------------|
| **Availability** | 20% | All three services start; UI status reaches Online or Directory mode within 2 min |
| **Directory accuracy** | 25% | IT/HR/contact answers match `kb.rs`; no invented phone numbers |
| **Assistant quality** | 20% | Drafting requests produce coherent, professional text |
| **Document workflow** | 20% | Upload → Q&A → edit → download works for PDF/DOCX under 10 MB |
| **Resilience** | 15% | Clear errors when Qwen down; KB still works; no raw “Internal Server Error” |

---

## 2. Prerequisites for evaluation

- Follow [SETUP.md](SETUP.md) completely on a clean machine  
- Record versions: OS, Python `python --version`, `cargo --version`, `java -version`  
- Note whether Qdrant is configured (RAG on/off)  
- Use a stopwatch for latency checks  

---

## 3. Automated tests

```powershell
cd ai-service
cargo test
```

**Expected:** all unit tests pass (NLP greetings, off-topic detection, KB search helpers).

This does **not** test HTTP or Python — only Rust logic.

---

## 4. Health checklist (before UI tests)

| # | Check | Command / action | Pass? |
|---|--------|------------------|-------|
| H1 | Spring up | `Invoke-RestMethod http://127.0.0.1:8080/api/health` | `backendReachable: true` |
| H2 | Rust up | `Invoke-RestMethod http://127.0.0.1:8090/health` | `status: ok` |
| H3 | Qwen ready | `Invoke-RestMethod http://127.0.0.1:8092/health` | `ready: true` |
| H4 | UI loads | Open http://127.0.0.1:8080/ | Page renders, no console errors |

---

## 5. Functional test cases (UI)

Record: **Pass / Fail**, **latency (rough)**, **notes**.

### 5.1 Greeting and small talk

| ID | Input | Expected | Engine (typical) |
|----|-------|----------|------------------|
| G1 | `hi` | Welcome message, no error | local |
| G2 | `what is up` | Friendly reply, not error | local / qwen |
| G3 | `how are you` | Brief polite reply | local / qwen |

### 5.2 Directory / policy (KB)

| ID | Input | Expected contains |
|----|-------|-------------------|
| D1 | `Who do I contact for IT support?` | IT Service Desk, **800** or documented number from KB |
| D2 | `How do I file a complaint?` | MyNCR, Ethics, Report a Concern (steps) |
| D3 | `Where is the main conference room?` | KB location answer or honest “don’t have” |

**Fail if:** fabricated phone numbers not in `kb.rs`.

### 5.3 Assistant / drafting (needs Qwen Online)

| ID | Input | Expected |
|----|-------|----------|
| A1 | `Help me write a professional email to my manager about a deadline extension.` | Structured email draft, professional tone |
| A2 | `Improve the tone of this: Sorry I was late.` | Rewritten, clearer text |

**Fail if:** 502/timeout with Qwen healthy; empty reply.

### 5.4 Off-topic handling

| ID | Input | Expected |
|----|-------|----------|
| O1 | `Tell me a joke about penguins` | Brief answer + redirect to work topics |
| O2 | `What is the capital of France?` | Does not pretend to be general-purpose encyclopedia; steers to work |

### 5.5 Document workflow (needs Qwen Online)

| ID | Step | Expected |
|----|------|----------|
| F1 | Upload `sample.txt` (< 10 MB) | “attached” confirmation, chip shows filename |
| F2 | `Summarize this in 3 bullet points` | Summary related to file content |
| F3 | `Improve tone and give me a download` | Preview in chat + download button |
| F4 | Click download | File opens; format matches (docx/pdf) |

| ID | Step | Expected |
|----|------|----------|
| F5 | Upload file > 10 MB | Clear “max 10 MB” message, no 500 error |

### 5.6 Degraded mode (optional)

Stop Qwen only (`stop.cmd` then `start-api.cmd` + `start-web.cmd`):

| ID | Input | Expected |
|----|-------|----------|
| R1 | `Who do I contact for IT support?` | Still works (KB) |
| R2 | `Help me write an email` | Message that AI/drafting unavailable |
| R3 | Upload document | Error that Qwen/document service required |

---

## 6. API-level tests (optional)

Scripted checks without UI — see [API.md](API.md).

```powershell
# Ask
$body = '{"message":"Who do I contact for IT support?","sessionId":"eval-1","history":[]}'
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:8090/api/ask -ContentType application/json -Body $body
```

---

## 7. Performance benchmarks (informal)

| Scenario | Target (local CPU) |
|----------|---------------------|
| KB hit (D1) | < 2 s end-to-end |
| First Qwen reply after cold start | < 30 s (model load excluded) |
| Qwen chat (A1) | < 60 s |
| Small doc upload + extract | < 30 s |

Record your hardware; GPU speeds up Qwen significantly.

---

## 8. Security / compliance checklist (internal)

| Item | Status |
|------|--------|
| Binds localhost by default | |
| No auth on demo UI | Document for production |
| `.env` not in git | |
| LLM instructed not to invent policy numbers | |
| Upload size capped at 10 MB | |

---

## 9. Evaluation report template

```markdown
## NCR AI Desk — Evaluation Report

**Evaluator:**  
**Date:**  
**Environment:** Windows / RAM / CPU / GPU  
**Qdrant RAG:** Yes / No  
**Model:** Qwen2.5-0.5B-Instruct in qwen-model/

### Summary
- Overall: Pass / Partial / Fail
- Best feature:
- Biggest gap:

### Health (Section 4)
H1–H4: ...

### Functional (Section 5)
G1–G3: ...
D1–D3: ...
A1–A2: ...
F1–F5: ...

### Automated tests
cargo test: pass/fail

### Recommendation
Deploy internally / Needs work on: ...
```

---

## 10. Related docs

- [SETUP.md](SETUP.md) — install from scratch  
- [ARCHITECTURE.md](ARCHITECTURE.md) — how components interact  
- [TRAINING.md](TRAINING.md) — changing KB content for your evaluation scenario  
