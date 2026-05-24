//! LLM: Perplexity Sonar (live web) + optional KB context from Qdrant.

use crate::perplexity;

const LIVE_WEB_RULES: &str = "\nYou can use current web information for weather, news, sports scores, \
     market data, and other real-time facts. Cite approximate dates when relevant. \
     For internal NCR directory facts (rooms, extensions), prefer the knowledge snippets provided; \
     do not invent NCR phone numbers or room codes.";

/// `auto` (default): use Perplexity when API key is set. `on`: require it. `off`: KB-only.
pub fn use_llm_mode() -> String {
    std::env::var("AI_DESK_USE_LLM")
        .unwrap_or_else(|_| "auto".into())
        .to_lowercase()
}

pub fn llm_explicitly_off() -> bool {
    matches!(use_llm_mode().as_str(), "off" | "false" | "0" | "no")
}

pub fn llm_explicitly_on() -> bool {
    matches!(use_llm_mode().as_str(), "on" | "true" | "1" | "yes")
}

fn assistant_max_tokens() -> u32 {
    std::env::var("PERPLEXITY_ASSISTANT_MAX_TOKENS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1024)
}

fn document_max_tokens() -> u32 {
    std::env::var("PERPLEXITY_DOCUMENT_MAX_TOKENS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4096)
}

fn directory_max_tokens() -> u32 {
    std::env::var("PERPLEXITY_DIRECTORY_MAX_TOKENS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(512)
}

pub async fn document_llm_unavailable_message() -> String {
    if perplexity::is_configured().await {
        "Document analysis is temporarily unavailable. Please try again.".into()
    } else {
        "To ask questions about PDF or Word files, add PERPLEXITY_API_KEY to .env \
         (https://www.perplexity.ai/settings/api), then restart the desk."
            .into()
    }
}

pub async fn live_llm_available() -> bool {
    if llm_explicitly_off() {
        return false;
    }
    perplexity::is_configured().await
}

pub fn system_prompt() -> String {
    format!(
        "You are the NCR Tech Solutions AI Desk — an internal assistant for NCR employees.\n\
         NCR Tech Solutions delivers retail, banking, and hospitality technology.\n\
         Answer using the NCR knowledge snippets provided when they apply.\n\
         Rules:\n\
         - Give direct, friendly answers (phone numbers, room locations, who to contact, steps).\n\
         - If snippets do not contain the answer, say you do not have that information \
           and suggest People Operations, IT Service Desk, or their manager — do NOT invent \
           internal phone numbers, rooms, or policies.\n\
         - Keep answers concise unless listing steps.\n\
         - Use **bold** for key names, numbers, and room codes when helpful.{LIVE_WEB_RULES}"
    )
}

pub fn assistant_system_prompt() -> String {
    format!(
        "You are the NCR Tech Solutions AI Desk — a workplace copilot for NCR employees.\n\
         You CAN help with: writing and rewriting emails, brainstorming, explaining concepts, \
         step-by-step plans, outlines, and general work questions.\n\
         Rules:\n\
         - Be practical, clear, and professional (NCR: retail, banking, hospitality technology).\n\
         - Do NOT invent NCR phone numbers, room codes, or policies.\n\
         - Do not claim you sent email or accessed internal systems — you provide text and guidance only.\n\
         - Use **bold** sparingly; bullet lists when helpful.{LIVE_WEB_RULES}"
    )
}

pub fn casual_redirect_system_prompt() -> String {
    format!(
        "You are the NCR Tech Solutions AI Desk — internal assistant for NCR employees.\n\
         The employee asked something outside normal work scope.\n\
         Rules:\n\
         - Reply in 2–4 short sentences.\n\
         - If harmless, answer briefly; decline unsafe or inappropriate requests.\n\
         - End by inviting a work question (directory, documents, drafting, IT).{LIVE_WEB_RULES}"
    )
}

pub fn document_assistant_system_prompt() -> String {
    "You are the NCR Tech Solutions AI Desk document assistant. \
     The employee uploaded a PDF or Word file. Its extracted text is in the user message.\n\
     Rules:\n\
     - Answer ONLY from the attached document unless they ask to compare with outside info.\n\
     - Do NOT search the web for summaries, form fields, or document Q&A.\n\
     - Quote sections or page themes when helpful.\n\
     - For summarize / explain / list action items: reply conversationally — do NOT dump the whole file.\n\
     - For improve / rewrite / edit / export requests: output the FULL revised document in plain text (# headings, - bullets).\n\
     - Do NOT wrap the document in markdown code fences.\n\
     - Preserve names, numbers, and dates unless asked to change them."
        .to_string()
}

pub fn document_edit_system_prompt() -> String {
    "You are the NCR Tech Solutions AI Desk document editor.\n\
     Rewrite the attached PDF/Word content per the employee's instructions.\n\
     Use ONLY the document text provided — do not add unrelated web content.\n\
     Output ONLY the full revised document as plain text. No preamble or code fences."
        .to_string()
}

pub async fn complete_assistant(
    user_message: &str,
    kb_context: &str,
    chat_history: &str,
) -> Result<(String, &'static str), String> {
    let history_block = if chat_history.trim().is_empty() {
        String::new()
    } else {
        format!("Recent conversation:\n{chat_history}\n\n")
    };
    let user_content = if kb_context.trim().is_empty() {
        format!("{history_block}Employee request:\n{user_message}")
    } else {
        format!(
            "{history_block}Optional NCR reference (use only if relevant):\n---\n{kb_context}\n---\n\n\
             Employee request:\n{user_message}"
        )
    };
    let reply = perplexity::complete(
        &assistant_system_prompt(),
        &user_content,
        Some(assistant_max_tokens()),
    )
    .await?;
    Ok((reply, "perplexity"))
}

pub async fn complete_casual_redirect(
    user_message: &str,
    chat_history: &str,
) -> Result<(String, &'static str), String> {
    let history_block = if chat_history.trim().is_empty() {
        String::new()
    } else {
        format!("Recent conversation:\n{chat_history}\n\n")
    };
    let user_content = format!("{history_block}Employee message:\n{user_message}");
    let reply = perplexity::complete(
        &casual_redirect_system_prompt(),
        &user_content,
        Some(assistant_max_tokens()),
    )
    .await?;
    Ok((reply, "perplexity"))
}

pub async fn complete_document_chat(
    user_message: &str,
    document_context: &str,
    chat_history: &str,
    kb_context: &str,
) -> Result<(String, &'static str), String> {
    let history_block = if chat_history.trim().is_empty() {
        String::new()
    } else {
        format!("Recent conversation:\n{chat_history}\n\n")
    };
    let kb_block = if kb_context.trim().is_empty() {
        String::new()
    } else {
        format!("Optional NCR reference:\n---\n{kb_context}\n---\n\n")
    };
    let user_content = format!(
        "{history_block}{kb_block}{document_context}\n\nEmployee message:\n{user_message}"
    );
    let reply = perplexity::complete_for_documents(
        &document_assistant_system_prompt(),
        &user_content,
        Some(document_max_tokens()),
    )
    .await?;
    Ok((reply, "perplexity"))
}

pub async fn complete_document_edit(
    instructions: &str,
    document_text: &str,
) -> Result<(String, &'static str), String> {
    let user_content = format!(
        "Current document:\n---\n{document_text}\n---\n\nInstructions:\n{instructions}\n\n\
         Output the complete revised document:"
    );
    let reply = perplexity::complete_for_documents(
        &document_edit_system_prompt(),
        &user_content,
        Some(document_max_tokens()),
    )
    .await?;
    Ok((reply, "perplexity"))
}

/// Directory / KB-backed questions with live web when needed.
pub async fn complete(user_message: &str, kb_context: &str) -> Result<(String, &'static str), String> {
    let user_content = if kb_context.trim().is_empty() {
        format!("Employee question:\n{user_message}")
    } else {
        format!(
            "NCR Tech Solutions knowledge (use for internal directory facts):\n---\n{kb_context}\n---\n\n\
             Employee question:\n{user_message}"
        )
    };
    let reply = perplexity::complete(
        &system_prompt(),
        &user_content,
        Some(directory_max_tokens()),
    )
    .await?;
    Ok((reply, "perplexity"))
}

pub async fn should_answer_with_llm() -> bool {
    if llm_explicitly_off() {
        return false;
    }
    if llm_explicitly_on() {
        return perplexity::is_configured().await;
    }
    perplexity::is_configured().await
}
