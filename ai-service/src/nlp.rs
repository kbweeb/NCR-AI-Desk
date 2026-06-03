use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

pub fn token_regex() -> &'static Regex {
    static TOKEN_RE: OnceLock<Regex> = OnceLock::new();
    TOKEN_RE.get_or_init(|| Regex::new(r"[A-Za-z0-9']+").expect("valid regex"))
}

pub fn stop_words() -> HashSet<&'static str> {
    [
        "the", "a", "an", "is", "it", "to", "for", "and", "or", "of", "on", "in", "at", "with",
        "that", "this", "i", "we", "you", "my", "our", "your", "be", "as", "are", "was", "were",
        "by", "from", "do", "does", "did", "can", "could", "would", "should", "what", "where",
        "who", "how", "when", "which", "me", "tell", "please", "need", "want", "get", "know",
    ]
    .into_iter()
    .collect()
}

pub fn tokenize(text: &str) -> Vec<String> {
    token_regex()
        .find_iter(&text.to_lowercase())
        .map(|m| m.as_str().to_string())
        .collect()
}

pub fn token_set(text: &str) -> HashSet<String> {
    tokenize(text).into_iter().collect()
}

pub fn extract_keywords(text: &str, top_n: usize) -> Vec<String> {
    let stop = stop_words();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for token in tokenize(text) {
        if token.len() < 3 || stop.contains(token.as_str()) {
            continue;
        }
        *counts.entry(token).or_insert(0) += 1;
    }
    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    pairs.into_iter().take(top_n).map(|(w, _)| w).collect()
}

/// Classify employee question intent.
pub fn classify_intent(text: &str) -> (&'static str, f32) {
    let tokens = tokenize(text);
    let joined = tokens.join(" ");

    let rules: Vec<(&str, Vec<&str>, f32)> = vec![
        (
            "contact_lookup",
            vec![
                "phone",
                "telephone",
                "number",
                "call",
                "email",
                "contact",
                "reach",
                "extension",
                "dial",
            ],
            1.0,
        ),
        (
            "location_lookup",
            vec![
                "where", "location", "room", "building", "floor", "file", "filing", "drop", "mail",
                "desk", "office",
            ],
            1.0,
        ),
        (
            "escalation",
            vec![
                "complaint",
                "complaints",
                "grievance",
                "harassment",
                "ethics",
                "compliance",
                "report",
                "concern",
                "anonymous",
            ],
            0.9,
        ),
        (
            "procedure",
            vec![
                "how", "submit", "request", "apply", "pto", "vacation", "expense", "process",
                "steps",
            ],
            0.85,
        ),
        (
            "it_support",
            vec![
                "password", "vpn", "computer", "laptop", "device", "login", "it",
            ],
            0.9,
        ),
    ];

    let mut best = ("general", 0f32);
    for (intent, keywords, weight) in rules {
        let hits: usize = keywords
            .iter()
            .map(|kw| {
                if joined.contains(kw) {
                    1
                } else {
                    tokens.iter().filter(|t| t.contains(kw)).count()
                }
            })
            .sum();
        let score = hits as f32 * weight;
        if score > best.1 {
            best = (intent, score);
        }
    }

    let confidence = if best.1 < 0.5 {
        0.35
    } else {
        (0.45 + best.1 * 0.12).min(0.95)
    };
    (best.0, confidence)
}

/// Writing, reasoning, and task-help requests (not directory / policy lookup).
pub fn is_assistant_request(text: &str) -> bool {
    let lower = text.to_lowercase();

    if looks_like_directory_lookup(&lower) {
        return false;
    }

    const PHRASES: &[&str] = &[
        "help me write",
        "help me draft",
        "help me create",
        "help me prepare",
        "help me figure",
        "help me understand",
        "help me solve",
        "help me plan",
        "help me decide",
        "help me explain",
        "help me summarize",
        "help me improve",
        "help me with this",
        "help me with my",
        "can you write",
        "can you draft",
        "can you help me write",
        "can you help me figure",
        "can you explain",
        "can you summarize",
        "write this",
        "rewrite this",
        "proofread",
        "brainstorm",
        "do this for me",
        "do it for me",
        "figure out",
        "work through",
        "step by step",
        "draft an email",
        "draft a message",
        "draft a",
        "compose a",
        "wording for",
        "make this sound",
        "improve this",
        "improve the tone",
        "improve the wording",
        "make it shorter",
        "make this shorter",
        "shorter version",
        "polish this",
        "rewrite this",
        "draft a letter",
        "draft a email",
        "draft an email",
        "write a letter",
        "template for",
        "outline for",
        "pros and cons",
        "compare these",
        "analyze this",
        "what should i say",
        "how should i phrase",
    ];

    if PHRASES.iter().any(|p| lower.contains(p)) {
        return true;
    }

    if lower.contains("help me") {
        const ACTION: &[&str] = &[
            "write",
            "draft",
            "create",
            "build",
            "make",
            "prepare",
            "think",
            "decide",
            "choose",
            "analyze",
            "compare",
            "explain",
            "solve",
            "plan",
            "email",
            "message",
            "letter",
            "report",
            "presentation",
            "slide",
        ];
        return ACTION.iter().any(|a| lower.contains(a));
    }

    false
}

/// Personal, casual, or general-knowledge questions outside NCR work scope.
pub fn is_off_topic_casual(text: &str) -> bool {
    if is_assistant_request(text) || is_work_scoped(text) {
        return false;
    }

    let lower = text.to_lowercase();
    if looks_like_directory_lookup(&lower) {
        return false;
    }

    let (intent, confidence) = classify_intent(text);
    if intent != "general" && confidence >= 0.5 {
        return false;
    }

    const CASUAL: &[&str] = &[
        "tell me a joke",
        "make me laugh",
        "roast me",
        "you're unhinged",
        "ignore your instructions",
        "pretend you are",
        "act like",
        "write a poem about",
        "write a song about",
        "who would win",
        "what is the meaning of life",
        "capital of",
        "recipe for",
        "dating advice",
        "relationship advice",
        "horoscope",
        "zodiac",
        "football game",
        "nba",
        "minecraft",
        "fortnite",
        "movie recommendation",
        "netflix",
        "celebrity",
        "conspiracy",
        "political opinion",
        "hot take",
        "rant about",
        "break into",
        "hack into",
        "illegal",
        "weapon",
        "drug",
        "medical diagnosis",
        "legal advice",
        "investment tip",
        "crypto tip",
        "lottery numbers",
        "pick a number",
        "truth or dare",
        "never have i ever",
        "would you rather",
        "rate my",
        "am i ugly",
        "fight club",
    ];

    if CASUAL.iter().any(|p| lower.contains(p)) {
        return true;
    }

    // Short non-work chatter
    if lower.len() < 48
        && !lower.contains("ncr")
        && !lower.contains("work")
        && !lower.contains("office")
        && tokenize(text).len() <= 6
    {
        let chatter = [
            "lol",
            "lmao",
            "bruh",
            "test",
            "testing",
            "hi there",
            "sup",
            "yo",
            "what is up",
            "what's up",
            "whats up",
            "how are you",
            "how are you doing",
            "how's it going",
            "hows it going",
        ];
        if chatter.iter().any(|c| lower == *c) {
            return true;
        }
        if is_small_talk_greeting(text) {
            return true;
        }
    }

    false
}

/// Hi / hello / "what's up" style messages (including light typos).
pub fn is_small_talk_greeting(text: &str) -> bool {
    let normalized = normalize_chatter(text);
    if normalized.is_empty() {
        return false;
    }
    const EXACT: &[&str] = &[
        "hi",
        "hello",
        "hey",
        "hi there",
        "hello there",
        "good morning",
        "good afternoon",
        "good evening",
        "help",
        "start",
        "yo",
        "sup",
        "whats up",
        "what is up",
        "what s up",
        "how are you",
        "how are you doing",
        "hows it going",
        "how s it going",
        "so whats up",
        "so what is up",
        "so whats is up",
    ];
    if EXACT.iter().any(|g| normalized == *g) {
        return true;
    }
    let words: Vec<&str> = normalized.split_whitespace().collect();
    if words.len() > 8 {
        return false;
    }
    let has_greet = words
        .iter()
        .any(|w| matches!(*w, "hi" | "hello" | "hey" | "yo" | "sup"));
    let has_up = normalized.contains("whats up") || normalized.contains("what is up");
    has_greet || has_up
}

fn normalize_chatter(text: &str) -> String {
    let lower = text.trim().to_lowercase();
    let cleaned: String = lower
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect();
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_work_scoped(text: &str) -> bool {
    let lower = text.to_lowercase();
    const WORK: &[&str] = &[
        "ncr",
        "work",
        "office",
        "manager",
        "colleague",
        "coworker",
        "client",
        "customer",
        "project",
        "deadline",
        "sprint",
        "ticket",
        "jira",
        "concur",
        "expense",
        "pto",
        "vacation request",
        "meeting",
        "presentation",
        "report",
        "email",
        "vpn",
        "password",
        "deploy",
        "retail",
        "banking",
        "hospitality",
        "myncr",
        "department",
        "stakeholder",
        "onboarding",
        "payroll",
        "benefits",
        "it desk",
        "service desk",
        "complaint",
        "policy",
        "procedure",
        "hq",
        "atlanta",
    ];
    WORK.iter().any(|w| lower.contains(w))
}

fn looks_like_directory_lookup(lower: &str) -> bool {
    lower.contains("phone number")
        || lower.contains("telephone")
        || lower.contains("extension")
        || lower.contains("where do i file")
        || lower.contains("who do i contact")
        || lower.contains("who do i send a complaint")
        || lower.contains("room ")
        || lower.contains("mailroom")
        || (lower.contains("ncr") && lower.contains("phone"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_writing_help() {
        assert!(is_assistant_request(
            "Help me write an email to my manager about PTO"
        ));
    }

    #[test]
    fn detects_figure_out() {
        assert!(is_assistant_request(
            "Can you help me figure out the best way to structure this report?"
        ));
    }

    #[test]
    fn not_assistant_for_phone_lookup() {
        assert!(!is_assistant_request(
            "What is the Business Solutions phone number?"
        ));
    }

    #[test]
    fn detects_off_topic_joke() {
        assert!(is_off_topic_casual("tell me a joke about penguins"));
    }

    #[test]
    fn work_question_not_off_topic() {
        assert!(!is_off_topic_casual("How do I request PTO through MyNCR?"));
    }

    #[test]
    fn small_talk_greetings() {
        assert!(is_small_talk_greeting("what is up"));
        assert!(is_small_talk_greeting("so whats is up"));
        assert!(!is_small_talk_greeting("What is the PTO policy?"));
    }
}
