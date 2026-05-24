use crate::documents::{self, DocumentArtifact};
use crate::kb::{entries, KbEntry};
use crate::llm;
use crate::nlp::{
    classify_intent, extract_keywords, is_assistant_request, is_off_topic_casual,
    is_small_talk_greeting, token_set,
};
use crate::qdrant_rag;
use crate::session::{self, ChatTurn};
use serde::Serialize;
use std::cmp::Ordering;

pub struct AskContext<'a> {
    pub message: &'a str,
    pub session_id: Option<&'a str>,
    pub history: &'a [ChatTurn],
    pub document_id: Option<&'a str>,
}

#[derive(Serialize, Clone)]
pub struct SourceRef {
    pub id: String,
    pub title: String,
    pub category: String,
    pub score: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AskResponse {
    pub reply: String,
    pub intent: String,
    pub confidence: f32,
    /// `local` / `rag` = KB; `perplexity` = live web LLM.
    pub engine: String,
    pub sources: Vec<SourceRef>,
    pub suggested_follow_ups: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_artifact: Option<DocumentArtifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_edit_preview: Option<String>,
}

fn entry_search_text(entry: &KbEntry) -> String {
    format!(
        "{} {} {} {}",
        entry.title,
        entry.body,
        entry.category,
        entry.tags.join(" ")
    )
}

fn category_intent_bonus(intent: &str, category: &str) -> f32 {
    match (intent, category) {
        ("contact_lookup", "contacts") => 0.45,
        ("location_lookup", "locations") => 0.45,
        ("escalation", "escalation") => 0.45,
        ("procedure", "procedures") => 0.35,
        ("it_support", "it") => 0.4,
        _ => 0.0,
    }
}

pub fn score_entry(
    entry: &KbEntry,
    query_tokens: &std::collections::HashSet<String>,
    intent: &str,
) -> f32 {
    let text = entry_search_text(entry);
    let entry_tokens = token_set(&text);
    let overlap = query_tokens.intersection(&entry_tokens).count();
    if overlap == 0 {
        return 0.0;
    }

    let tag_hits = entry
        .tags
        .iter()
        .filter(|tag| query_tokens.contains(**tag))
        .count();

    let tag_bonus = tag_hits as f32 * 0.35;

    let title_tokens = token_set(entry.title);
    let title_overlap = query_tokens.intersection(&title_tokens).count() as f32 * 0.25;

    let base = overlap as f32 / query_tokens.len().max(1) as f32;
    let category_bonus = category_intent_bonus(intent, entry.category);

    base + tag_bonus + title_overlap + category_bonus
}

/// Qdrant vector search when configured; otherwise lexical KB search.
pub async fn search_kb(message: &str, intent: &str, limit: usize) -> (Vec<(KbEntry, f32)>, &'static str) {
    if qdrant_rag::is_configured() && qdrant_rag::is_ready().await {
        match qdrant_rag::search(message, limit).await {
            Ok(hits) if !hits.is_empty() => return (hits, "rag"),
            Ok(_) => {}
            Err(e) => eprintln!("Qdrant search fallback to local: {e}"),
        }
    }
    (search_kb_local(message, intent, limit), "local")
}

pub fn search_kb_local(message: &str, intent: &str, limit: usize) -> Vec<(KbEntry, f32)> {
    let query_tokens = token_set(message);
    if query_tokens.is_empty() {
        return vec![];
    }

    let mut scored: Vec<(KbEntry, f32)> = entries()
        .into_iter()
        .map(|entry| {
            let score = score_entry(&entry, &query_tokens, intent);
            (entry, score)
        })
        .filter(|(_, s)| *s > 0.12)
        .collect();

    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.0.id.cmp(b.0.id))
    });
    scored.into_iter().take(limit).collect()
}

fn follow_ups_for_intent(intent: &str) -> Vec<String> {
    match intent {
        "contact_lookup" => vec![
            "NCR IT Service Desk phone number".into(),
            "Business Solutions department contact".into(),
        ],
        "location_lookup" => vec![
            "Where do I file a physical report?".into(),
            "Mailroom location".into(),
        ],
        "escalation" => vec![
            "How do I submit a complaint electronically?".into(),
        ],
        "procedure" => vec![
            "How do I request PTO?".into(),
            "How do I submit an expense report?".into(),
        ],
        "it_support" => vec![
            "Reset my work password".into(),
            "VPN setup for remote work".into(),
        ],
        "assistant" => vec![
            "Help me write a professional email".into(),
            "Help me figure out how to structure a project update".into(),
        ],
        "document" => vec![
            "Summarize this document in bullet points".into(),
            "What are the key action items?".into(),
            "Improve the wording and export as PDF".into(),
        ],
        "document_edit" => vec![
            "Export as Word (.docx)".into(),
            "Make it more concise".into(),
        ],
        _ => vec![
            "Business Solutions phone number".into(),
            "Where do I file a physical report at Atlanta HQ?".into(),
        ],
    }
}

fn sources_from_matches(matches: &[(KbEntry, f32)]) -> Vec<SourceRef> {
    matches
        .iter()
        .map(|(e, s)| SourceRef {
            id: e.id.to_string(),
            title: e.title.to_string(),
            category: e.category.to_string(),
            score: *s,
        })
        .collect()
}

fn kb_fast_path_threshold() -> f32 {
    std::env::var("AI_DESK_FAST_KB_SCORE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.55)
}

fn force_llm_rewrite() -> bool {
    matches!(
        std::env::var("AI_DESK_ALWAYS_LLM")
            .unwrap_or_default()
            .to_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn kb_context_for_llm(matches: &[(KbEntry, f32)]) -> String {
    if matches.is_empty() {
        return String::new();
    }
    const MAX_BODY_CHARS: usize = 400;
    matches
        .iter()
        .take(2)
        .map(|(e, _)| {
            let body = e.body.replace("**", "");
            let body = if body.len() > MAX_BODY_CHARS {
                format!("{}…", &body[..MAX_BODY_CHARS])
            } else {
                body
            };
            format!("[{}]\n{body}\n", e.title)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn greeting_response() -> AskResponse {
    AskResponse {
        reply: "Welcome to the **NCR Tech Solutions AI Desk**. I can help with directory and \
                policy questions, PDFs, and drafting work content."
            .into(),
        intent: "greeting".into(),
        confidence: 1.0,
        engine: "local".into(),
        sources: vec![SourceRef {
            id: "welcome".into(),
            title: "What the NCR AI Desk can help with".into(),
            category: "general".into(),
            score: 1.0,
        }],
        suggested_follow_ups: follow_ups_for_intent("general"),
        document_artifact: None,
        active_document_id: None,
        document_edit_preview: None,
    }
}

/// KB search + templated reply (no LLM). Used for tests and fallback.
pub fn answer_local(message: &str) -> AskResponse {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return AskResponse {
            reply: "Ask a work question — for example, a department phone number or where to file a report."
                .into(),
            intent: "empty".into(),
            confidence: 1.0,
            engine: "local".into(),
            sources: vec![],
            suggested_follow_ups: follow_ups_for_intent("general"),
            document_artifact: None,
            active_document_id: None,
            document_edit_preview: None,
        };
    }

    if is_small_talk_greeting(trimmed) {
        return greeting_response();
    }

    let (intent, confidence) = classify_intent(trimmed);
    let matches = search_kb_local(trimmed, intent, 3);

    if let Some((best, _score)) = matches.first() {
        let reply = format!("**{}**\n\n{}", best.title, best.body);

        return AskResponse {
            reply,
            intent: intent.to_string(),
            confidence,
            engine: "local".into(),
            suggested_follow_ups: follow_ups_for_intent(intent),
            sources: sources_from_matches(&matches),
            document_artifact: None,
            active_document_id: None,
            document_edit_preview: None,
        };
    }

    let _keywords = extract_keywords(trimmed, 4);

    AskResponse {
        reply: "I don't have that in the directory yet. Try a department name, office location, \
                procedure, or IT topic — or contact People Operations or the IT Service Desk."
            .into(),
        intent: intent.to_string(),
        confidence,
        engine: "local".into(),
        sources: vec![],
        suggested_follow_ups: follow_ups_for_intent(intent),
        document_artifact: None,
        active_document_id: None,
        document_edit_preview: None,
    }
}

fn finish_response(
    mut response: AskResponse,
    ctx: &AskContext<'_>,
    user_message: &str,
) -> AskResponse {
    session::append_turns(ctx.session_id, user_message, &response.reply);
    if let Some(doc_id) = ctx.document_id {
        response.active_document_id = Some(doc_id.to_string());
    }
    response
}

async fn answer_with_document(ctx: &AskContext<'_>) -> AskResponse {
    let trimmed = ctx.message.trim();
    let session_id = ctx.session_id.unwrap_or("");
    let doc_id = ctx.document_id.unwrap_or("");
    let history = session::merge_history(ctx.session_id, ctx.history);
    let history_text = session::format_history_for_llm(&history);

    let Some(doc) = documents::get_for_session(doc_id, session_id) else {
        return AskResponse {
            reply: "I don't have that document in this session. Attach it again."
                .into(),
            intent: "document".into(),
            confidence: 0.5,
            engine: "local".into(),
            sources: vec![],
            suggested_follow_ups: vec![],
            document_artifact: None,
            active_document_id: None,
            document_edit_preview: None,
        };
    };

    let doc_ctx = documents::document_context_for_llm(&doc);
    let (matches, _) = search_kb(trimmed, "assistant", 2).await;
    let kb_context = kb_context_for_llm(&matches);

    if !llm::should_answer_with_llm().await {
        let msg = llm::document_llm_unavailable_message().await;
        return finish_response(
            AskResponse {
                reply: msg,
                intent: "document".into(),
                confidence: 0.5,
                engine: "local".into(),
                sources: vec![],
                suggested_follow_ups: follow_ups_for_intent("document"),
                document_artifact: None,
                active_document_id: Some(doc_id.to_string()),
                document_edit_preview: None,
            },
            ctx,
            trimmed,
        );
    }

    if documents::is_document_edit_request(trimmed) {
        match llm::complete_document_edit(trimmed, &doc.current_text).await {
            Ok((edited, engine)) => {
                match documents::apply_edit(doc_id, session_id, trimmed, &edited).await {
                    Ok((artifact, preview)) => finish_response(
                        AskResponse {
                            reply: "Your revised document is below. Download the file when you're ready."
                                .into(),
                            intent: "document_edit".into(),
                            confidence: 0.92,
                            engine: engine.to_string(),
                            sources: vec![],
                            suggested_follow_ups: follow_ups_for_intent("document_edit"),
                            document_artifact: Some(artifact),
                            active_document_id: Some(doc_id.to_string()),
                            document_edit_preview: Some(preview),
                        },
                        ctx,
                        trimmed,
                    ),
                    Err(_err) => finish_response(
                        AskResponse {
                            reply: "I drafted the changes but could not create the export file. Please try again."
                                .to_string(),
                            intent: "document_edit".into(),
                            confidence: 0.4,
                            engine: engine.to_string(),
                            sources: vec![],
                            suggested_follow_ups: follow_ups_for_intent("document_edit"),
                            document_artifact: None,
                            active_document_id: Some(doc_id.to_string()),
                            document_edit_preview: Some(documents::chat_preview(&edited)),
                        },
                        ctx,
                        trimmed,
                    ),
                }
            }
            Err(err) => finish_response(
                AskResponse {
                    reply: if err.contains("PERPLEXITY_API_KEY") {
                        err
                    } else {
                        format!("I couldn't edit the document: {err}")
                    },
                    intent: "document_edit".into(),
                    confidence: 0.4,
                    engine: "local".into(),
                    sources: vec![],
                    suggested_follow_ups: follow_ups_for_intent("document_edit"),
                    document_artifact: None,
                    active_document_id: Some(doc_id.to_string()),
                    document_edit_preview: None,
                },
                ctx,
                trimmed,
            ),
        }
    } else {
        match llm::complete_document_chat(trimmed, &doc_ctx, &history_text, &kb_context).await {
            Ok((reply, engine)) => finish_response(
                AskResponse {
                    reply,
                    intent: "document".into(),
                    confidence: 0.9,
                    engine: engine.to_string(),
                    sources: vec![],
                    suggested_follow_ups: follow_ups_for_intent("document"),
                    document_artifact: None,
                    active_document_id: Some(doc_id.to_string()),
                    document_edit_preview: None,
                },
                ctx,
                trimmed,
            ),
            Err(err) => finish_response(
                AskResponse {
                    reply: if err.contains("PERPLEXITY_API_KEY") {
                        err
                    } else {
                        format!("I couldn't analyze the document: {err}")
                    },
                    intent: "document".into(),
                    confidence: 0.4,
                    engine: "local".into(),
                    sources: vec![],
                    suggested_follow_ups: follow_ups_for_intent("document"),
                    document_artifact: None,
                    active_document_id: Some(doc_id.to_string()),
                    document_edit_preview: None,
                },
                ctx,
                trimmed,
            ),
        }
    }
}

/// Answer with optional local LLM (Ollama) + RAG over the knowledge base.
pub async fn answer(ctx: AskContext<'_>) -> AskResponse {
    let trimmed = ctx.message.trim();
    if ctx.document_id.is_some() {
        return answer_with_document(&ctx).await;
    }

    let history = session::merge_history(ctx.session_id, ctx.history);
    let history_text = session::format_history_for_llm(&history);

    if trimmed.is_empty() || is_small_talk_greeting(trimmed) {
        return finish_response(greeting_response(), &ctx, trimmed);
    }

    if is_off_topic_casual(trimmed) {
        let follow_ups = follow_ups_for_intent("general");
        if !llm::should_answer_with_llm().await {
            return finish_response(
                AskResponse {
                    reply: "I'm here for NCR work — directory, documents, drafting, and IT. \
                            What do you need for your job today?"
                        .into(),
                    intent: "off_topic".into(),
                    confidence: 0.85,
                    engine: "local".into(),
                    sources: vec![],
                    suggested_follow_ups: follow_ups,
                    document_artifact: None,
                    active_document_id: None,
                    document_edit_preview: None,
                },
                &ctx,
                trimmed,
            );
        }
        return match llm::complete_casual_redirect(trimmed, &history_text).await {
            Ok((reply, engine)) => finish_response(
                AskResponse {
                    reply,
                    intent: "off_topic".into(),
                    confidence: 0.85,
                    engine: engine.to_string(),
                    sources: vec![],
                    suggested_follow_ups: follow_ups,
                    document_artifact: None,
                    active_document_id: None,
                    document_edit_preview: None,
                },
                &ctx,
                trimmed,
            ),
            Err(_) => finish_response(
                AskResponse {
                    reply: "I'm here for NCR work — directory, documents, drafting, and IT. \
                            What do you need for your job today?"
                        .into(),
                    intent: "off_topic".into(),
                    confidence: 0.5,
                    engine: "local".into(),
                    sources: vec![],
                    suggested_follow_ups: follow_ups,
                    document_artifact: None,
                    active_document_id: None,
                    document_edit_preview: None,
                },
                &ctx,
                trimmed,
            ),
        };
    }

    let (mut intent, confidence) = classify_intent(trimmed);
    let (matches, retrieval) = search_kb(trimmed, intent, 3).await;
    let sources = sources_from_matches(&matches);
    let context = kb_context_for_llm(&matches);

    // Writing, reasoning, and task help — always use the assistant LLM path.
    if is_assistant_request(trimmed) {
        intent = "assistant";
        let follow_ups = follow_ups_for_intent(intent);

        if !llm::should_answer_with_llm().await {
            return finish_response(
                AskResponse {
                    reply: "Drafting and reasoning need the AI service, which is unavailable right now. \
                            Directory questions still work — try a phone number or office location."
                        .into(),
                    intent: intent.to_string(),
                    confidence: 0.9,
                    engine: "local".into(),
                    sources,
                    suggested_follow_ups: follow_ups,
                    document_artifact: None,
                    active_document_id: None,
                    document_edit_preview: None,
                },
                &ctx,
                trimmed,
            );
        }

        return match llm::complete_assistant(trimmed, &context, &history_text).await {
            Ok((reply, engine)) => finish_response(
                AskResponse {
                    reply,
                    intent: intent.to_string(),
                    confidence: 0.9,
                    engine: engine.to_string(),
                    sources,
                    suggested_follow_ups: follow_ups,
                    document_artifact: None,
                    active_document_id: None,
                    document_edit_preview: None,
                },
                &ctx,
                trimmed,
            ),
            Err(err) => {
                eprintln!("Assistant LLM error: {err}");
                finish_response(
                    AskResponse {
                        reply: "Drafting and reasoning are unavailable right now. \
                                 Try a directory question instead."
                            .to_string(),
                        intent: intent.to_string(),
                        confidence: 0.5,
                        engine: "local".into(),
                        sources,
                        suggested_follow_ups: follow_ups,
                        document_artifact: None,
                        active_document_id: None,
                        document_edit_preview: None,
                    },
                    &ctx,
                    trimmed,
                )
            }
        };
    }

    let follow_ups = follow_ups_for_intent(intent);

    // Strong KB / RAG hit: skip LLM (instant for directory facts).
    if let Some((best, score)) = matches.first() {
        if *score >= kb_fast_path_threshold() && !force_llm_rewrite() {
            let reply = format!("**{}**\n\n{}", best.title, best.body);
            let response = AskResponse {
                reply,
                intent: intent.to_string(),
                confidence,
                engine: retrieval.to_string(),
                sources,
                suggested_follow_ups: follow_ups,
                document_artifact: None,
                active_document_id: None,
                document_edit_preview: None,
            };
            return finish_response(response, &ctx, trimmed);
        }
    }

    if !llm::should_answer_with_llm().await {
        return answer_local(trimmed);
    }

    match llm::complete(trimmed, &context).await {
        Ok((reply, engine)) => finish_response(
            AskResponse {
                reply,
                intent: intent.to_string(),
                confidence,
                engine: engine.to_string(),
                sources,
                suggested_follow_ups: follow_ups,
                document_artifact: None,
                active_document_id: None,
                document_edit_preview: None,
            },
            &ctx,
            trimmed,
        ),
        Err(err) => {
            eprintln!("LLM fallback to local search: {err}");
            answer_local(trimmed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_business_phone() {
        let r = answer_local("What is the Business Solutions department phone number at NCR?");
        assert!(r.reply.contains("399-3220") || r.reply.contains("Business Solutions"));
        assert!(!r.sources.is_empty());
    }

    #[test]
    fn finds_physical_report_location() {
        let r = answer_local("Where do I file my physical report at Atlanta HQ?");
        assert!(r.reply.contains("1-G14") || r.reply.contains("Records"));
    }

    #[test]
    fn finds_complaint_escalation() {
        let r = answer_local("Who do I send a complaint to using my NCR laptop?");
        assert!(r.reply.contains("integrity") || r.reply.contains("MyNCR"));
    }
}
