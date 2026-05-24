//! In-memory conversation history per chat session.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

const MAX_TURNS: usize = 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
}

static SESSIONS: Mutex<Option<HashMap<String, Vec<ChatTurn>>>> = Mutex::new(None);

fn store() -> std::sync::MutexGuard<'static, Option<HashMap<String, Vec<ChatTurn>>>> {
    let mut guard = SESSIONS.lock().expect("session lock");
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

pub fn merge_history(session_id: Option<&str>, client_history: &[ChatTurn]) -> Vec<ChatTurn> {
    let mut history = if let Some(id) = session_id.filter(|s| !s.is_empty()) {
        let guard = store();
        guard
            .as_ref()
            .and_then(|m| m.get(id).cloned())
            .unwrap_or_default()
    } else {
        vec![]
    };

    if !client_history.is_empty() {
        history = client_history.to_vec();
    }

    if history.len() > MAX_TURNS {
        history = history[history.len() - MAX_TURNS..].to_vec();
    }
    history
}

pub fn append_turns(session_id: Option<&str>, user: &str, assistant: &str) {
    let Some(id) = session_id.filter(|s| !s.is_empty()) else {
        return;
    };
    let mut guard = store();
    let map = guard.as_mut().expect("session map");
    let turns = map.entry(id.to_string()).or_default();
    turns.push(ChatTurn {
        role: "user".into(),
        content: user.to_string(),
    });
    turns.push(ChatTurn {
        role: "assistant".into(),
        content: assistant.to_string(),
    });
    if turns.len() > MAX_TURNS {
        let drop = turns.len() - MAX_TURNS;
        turns.drain(0..drop);
    }
}

pub fn format_history_for_llm(history: &[ChatTurn]) -> String {
    if history.is_empty() {
        return String::new();
    }
    history
        .iter()
        .map(|t| {
            let label = if t.role == "assistant" {
                "Assistant"
            } else {
                "Employee"
            };
            format!("{label}: {}", t.content.trim())
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
