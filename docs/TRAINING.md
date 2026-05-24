# Training and customizing the AI Desk

This project does **not** require training a neural network for normal operation. You “teach” the desk by editing **facts** and **prompts**, not by fine-tuning weights.

**See also:** [PROJECT-STRUCTURE.md](PROJECT-STRUCTURE.md) · [CONFIGURATION.md](CONFIGURATION.md) · [EVALUATION.md](EVALUATION.md)

---

## 1. Company knowledge (primary method)

Edit **`ai-service/src/kb.rs`**.

Each entry is a `KbEntry`:

| Field | Purpose |
|-------|---------|
| `id` | Unique key (e.g. `phone-it`) |
| `title` | Short label shown to users |
| `body` | Full answer (Markdown-style `**bold**` allowed) |
| `category` | `contacts`, `locations`, `procedures`, `escalation`, `it`, `general` |
| `tags` | Words employees might type |

**Example:**

```rust
KbEntry {
    id: "phone-it",
    category: "it",
    title: "IT Service Desk",
    body: "Contact **NCR IT Service Desk** at +1 (800) 225-5627.",
    tags: &["it", "support", "helpdesk", "password", "laptop"],
},
```

**Apply changes:**

```powershell


.\start-api.cmd
# Or full stack: .\start.cmd
```

No GPU, no dataset. After restart, directory questions use the new facts (and RAG re-indexes on API startup if Qdrant is configured).

---

## 2. Behavior and tone (system prompts)

Edit **`ai-service/src/llm.rs`**.

| Function | Used for |
|----------|----------|
| `system_prompt()` | Directory answers with KB context |
| `assistant_system_prompt()` | Writing, email drafts, reasoning |
| `document_assistant_system_prompt()` | Q&A on uploaded files |
| `document_edit_system_prompt()` | Full document rewrite output |
| `casual_redirect_system_prompt()` | Off-topic small talk |

Rules already include: do not invent NCR phone numbers; stay professional; steer back to work.

---

## 3. Routing (when AI runs vs instant KB)

| Location | Controls |
|----------|----------|
| `ai-service/src/nlp.rs` | Intent rules, greetings, off-topic, assistant detection |
| `ai-service/src/chat.rs` | Fast-path KB score, document mode, fallbacks |
| Environment | See [CONFIGURATION.md](CONFIGURATION.md) |

| Setting | Effect |
|---------|--------|
| Default | Strong KB match → **instant** answer (no Qwen) |
| `AI_DESK_ALWAYS_LLM=1` | Always call Qwen for KB answers (slower, more paraphrased) |
| `AI_DESK_FAST_KB_SCORE` | Lower = more questions skip LLM |
| `QWEN_MAX_NEW_TOKENS` | Shorter Qwen replies |
| `AI_DESK_USE_LLM=off` | KB only; no drafting |

---

## 4. Optional: Qdrant RAG

1. Set `QDRANT_URL`, `QDRANT_API_KEY` in `.env`  
2. Restart Rust API — background task indexes `kb.rs` into Qdrant  
3. Search uses embeddings from Qwen `/embed`  

Improves matching when employees use varied wording. Not required for basic operation.

---

## 5. Optional: fine-tuning (advanced)

Fine-tuning **changes model weights**. Requirements:

- GPU (8GB+ VRAM for 0.5B LoRA typical)
- Hundreds+ of example Q&A pairs in JSONL

Tools: LLaMA-Factory, Unsloth, or Hugging Face `trl` + LoRA on `Qwen/Qwen2.5-0.5B-Instruct`.

After training, point `QWEN_MODEL_DIR` at the new folder.

**Recommendation:** Use **kb.rs + RAG + prompts** first. Fine-tune only for tone/language that prompts cannot fix.

---

## 6. Team workflow

1. HR/IT provides FAQ spreadsheet.  
2. Add entries to `kb.rs` (future: load from JSON/YAML).  
3. Restart API.  
4. Employees use UI — strong matches are instant; unclear questions use Qwen with retrieved snippets.

That is practical “training” without retraining the model.

---

## 7. UI copy and limits

| File | Change |
|------|--------|
| `src/main/resources/static/index.html` | Labels, suggestions, sidebar text |
| `src/main/resources/static/app.js` | Client behavior, timeouts |
| `application.properties` | Upload size, backend URL |

Restart Spring after UI edits: `.\start-web.cmd` or full `.\start.cmd`.
