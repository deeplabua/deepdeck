//! Claude Code hook payload → [`Event`]. Pure: no file system, no clock.
//!
//! Field names are taken from real payloads of Claude Code 2.1.316 (anonymized
//! samples in `tests/fixtures/`). Everything is read
//! leniently: a missing optional field degrades the event, it never fails the hook.

use std::path::PathBuf;

use jiff::Timestamp;
use serde_json::Value;

use crate::event::{Event, EventBody, PlanItem, Source, TaskStatus};

/// Max characters of a user prompt kept in the log.
pub const PROMPT_MAX_CHARS: usize = 200;
/// Max characters of an agent turn recap.
pub const RECAP_MAX_CHARS: usize = 240;
/// Max characters of a notification text.
const NOTIFICATION_MAX_CHARS: usize = 200;

/// A parsed hook call: where it happened and what to append.
#[derive(Debug, Clone, PartialEq)]
pub struct HookEvent {
    pub cwd: PathBuf,
    pub session: String,
    /// `hook_event_name` as sent by Claude Code (for logs and dumps).
    pub hook_name: String,
    pub event: Event,
}

#[derive(Debug, thiserror::Error)]
pub enum HookError {
    #[error("payload is not JSON")]
    Json(#[from] serde_json::Error),
    #[error("payload is not a JSON object")]
    NotObject,
    #[error("payload has no `{0}`")]
    Missing(&'static str),
}

/// Hook name of a raw payload, if it has one. Used to name dumps.
pub fn hook_name(raw: &[u8]) -> Option<String> {
    let v: Value = serde_json::from_slice(raw).ok()?;
    v.get("hook_event_name")?.as_str().map(str::to_owned)
}

/// Parse a hook payload. `Ok(None)` means "valid, but nothing for the board"
/// (another tool's `PostToolUse`, an unknown hook, an auth notification…).
pub fn parse_hook(raw: &[u8], now: Timestamp) -> Result<Option<HookEvent>, HookError> {
    let v: Value = serde_json::from_slice(raw)?;
    let obj = v.as_object().ok_or(HookError::NotObject)?;
    let hook_name =
        str_field(obj, "hook_event_name").ok_or(HookError::Missing("hook_event_name"))?;
    let session = str_field(obj, "session_id").ok_or(HookError::Missing("session_id"))?;
    let cwd = str_field(obj, "cwd").ok_or(HookError::Missing("cwd"))?;

    let body = match hook_name {
        "SessionStart" => Some(EventBody::AgentStarted {
            source: str_field(obj, "source").unwrap_or("startup").to_owned(),
            herdr_pane: None,
        }),
        "UserPromptSubmit" => str_field(obj, "prompt")
            .filter(|p| !is_system_message(p))
            .map(|p| EventBody::UserPrompt {
                text: truncate_chars(&clean_prompt(p), PROMPT_MAX_CHARS),
            }),
        "PostToolUse" => parse_tool_use(&v),
        "Notification" => parse_notification(&v),
        // SubagentStop is deliberately ignored: only the main agent's turn matters.
        "Stop" => Some(EventBody::AgentStopped {
            recap: str_field(obj, "last_assistant_message").and_then(recap),
        }),
        "SessionEnd" => Some(EventBody::AgentEnded {
            reason: str_field(obj, "reason").map(str::to_owned),
        }),
        _ => None,
    };

    Ok(body.map(|body| HookEvent {
        cwd: PathBuf::from(cwd),
        session: session.to_owned(),
        hook_name: hook_name.to_owned(),
        event: Event::new(now, Source::Claude, Some(session.to_owned()), body),
    }))
}

fn str_field<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a str> {
    obj.get(key)?.as_str()
}

fn parse_tool_use(v: &Value) -> Option<EventBody> {
    let input = &v["tool_input"];
    let response = &v["tool_response"];
    match v["tool_name"].as_str()? {
        "TaskCreate" => {
            // The id is assigned by Claude Code and only present in the response.
            let task_id =
                id_string(&response["task"]["id"]).or_else(|| id_string(&response["taskId"]))?;
            let subject = input["subject"]
                .as_str()
                .or_else(|| response["task"]["subject"].as_str())
                .map(|s| truncate_chars(&collapse_ws(s), RECAP_MAX_CHARS));
            let status = input["status"]
                .as_str()
                .map(TaskStatus::parse)
                .unwrap_or(TaskStatus::Pending);
            Some(EventBody::TaskUpsert {
                task_id,
                subject,
                status: Some(status),
            })
        }
        "TaskUpdate" => {
            let task_id = id_string(&input["taskId"]).or_else(|| id_string(&response["taskId"]))?;
            let subject = input["subject"]
                .as_str()
                .map(|s| truncate_chars(&collapse_ws(s), RECAP_MAX_CHARS));
            let status = input["status"].as_str().map(TaskStatus::parse);
            if subject.is_none() && status.is_none() {
                // Only description/owner/dependencies changed — not shown on the board.
                return None;
            }
            Some(EventBody::TaskUpsert {
                task_id,
                subject,
                status,
            })
        }
        "TodoWrite" => {
            let todos = input["todos"].as_array()?;
            let tasks = todos
                .iter()
                .filter_map(|t| {
                    let subject = t["content"].as_str().or_else(|| t["subject"].as_str())?;
                    Some(PlanItem {
                        subject: truncate_chars(&collapse_ws(subject), RECAP_MAX_CHARS),
                        status: TaskStatus::parse(t["status"].as_str().unwrap_or("pending")),
                    })
                })
                .collect();
            Some(EventBody::PlanSnapshot { tasks })
        }
        _ => None,
    }
}

/// Task ids are strings in current Claude Code, but accept numbers too.
fn id_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn parse_notification(v: &Value) -> Option<EventBody> {
    let reason = v["notification_type"].as_str();
    // Only notifications that mean "the agent is blocked on you" go to the board.
    // Missing type (older versions) is treated as waiting.
    match reason {
        None | Some("permission_prompt" | "idle_prompt" | "elicitation_dialog") => {}
        Some(_) => return None,
    }
    let text = v["message"].as_str().unwrap_or("Claude is waiting for you");
    Some(EventBody::AgentWaiting {
        text: truncate_chars(&collapse_ws(text), NOTIFICATION_MAX_CHARS),
        reason: reason.map(str::to_owned),
    })
}

/// Short recap of an assistant message: first paragraph, at most two sentences,
/// at most [`RECAP_MAX_CHARS`] characters, without markdown emphasis.
pub fn recap(message: &str) -> Option<String> {
    let para = message
        .split("\n\n")
        .map(str::trim)
        .find(|p| !p.is_empty() && !p.starts_with("```"))?;
    let text = collapse_ws(&para.replace(['*', '`'], "").replace("# ", ""));
    let text = text.trim_start_matches('#').trim();
    if text.is_empty() {
        return None;
    }
    let mut out = String::new();
    for (i, sentence) in sentences(text).into_iter().enumerate() {
        if i == 2 || (i > 0 && out.chars().count() + 1 + sentence.chars().count() > RECAP_MAX_CHARS)
        {
            break;
        }
        if i > 0 {
            out.push(' ');
        }
        out.push_str(sentence);
    }
    Some(truncate_chars(&out, RECAP_MAX_CHARS))
}

/// Split after `.`, `!`, `?` or `…` followed by whitespace.
fn sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if matches!(c, '.' | '!' | '?' | '…')
            && let Some(&(_, next)) = chars.peek()
            && next.is_whitespace()
        {
            let end = i + c.len_utf8();
            out.push(text[start..end].trim());
            start = end;
        }
    }
    let rest = text[start..].trim();
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

/// Tags Claude Code uses for messages it injects as a "prompt" itself
/// (background task results, local command output). Not something the user said.
const SYSTEM_TAGS: [&str; 6] = [
    "<task-notification>",
    "<system-reminder>",
    "<local-command-stdout>",
    "<local-command-stderr>",
    "<local-command-caveat>",
    "<bash-notification>",
];

/// True for a prompt Claude Code generated rather than the user typed.
pub fn is_system_message(prompt: &str) -> bool {
    let p = prompt.trim_start();
    SYSTEM_TAGS.iter().any(|t| p.starts_with(t))
}

/// What the user typed, without Claude Code's wrappers: `[Image #N]`
/// placeholders and `<pasted_content …>` tags (their content stays).
/// Idempotent, so the fold can apply it to old log lines too.
pub fn clean_prompt(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = [rest.find("<pasted_content"), rest.find("</pasted_content")]
        .into_iter()
        .flatten()
        .min()
    {
        out.push_str(&rest[..i]);
        match rest[i..].find('>') {
            Some(end) => rest = &rest[i + end + 1..],
            None => rest = "",
        }
        out.push(' ');
    }
    out.push_str(rest);
    collapse_ws(&strip_image_refs(&out))
}

/// Drop Claude Code's `[Image #N]` placeholders: they carry no meaning in a
/// 200-character excerpt. A prompt that was only images becomes `[image]`.
pub fn strip_image_refs(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("[Image #") {
        out.push_str(&rest[..i]);
        let after = &rest[i + "[Image #".len()..];
        let digits = after.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && after.as_bytes().get(digits) == Some(&b']') {
            rest = &after[digits + 1..];
        } else {
            out.push_str("[Image #");
            rest = after;
        }
    }
    out.push_str(rest);
    if out.trim().is_empty() && !s.trim().is_empty() {
        return "[image]".to_owned();
    }
    out
}

/// Collapse all whitespace runs (including newlines) into single spaces.
pub fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Truncate to `max` characters (not bytes), ending with `…` when cut.
pub fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.truncate(out.trim_end().len());
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_counts_chars_not_bytes() {
        assert_eq!(truncate_chars("привет", 6), "привет");
        assert_eq!(truncate_chars("привет мир", 7), "привет…");
        assert_eq!(truncate_chars("привет мир", 8), "привет…");
    }

    #[test]
    fn pasted_content_tags_are_dropped() {
        assert_eq!(
            clean_prompt(
                "<pasted_content id=\"32d0\"> ◐ DeepVault plan 3/5 </pasted_content id=\"32d0\">\n\nadd navigation"
            ),
            "◐ DeepVault plan 3/5 add navigation"
        );
        assert_eq!(clean_prompt("a < b and c > d"), "a < b and c > d");
        let once =
            clean_prompt("<pasted_content id=\"x\">hi</pasted_content id=\"x\"> [Image #2] ok");
        assert_eq!(once, "hi ok");
        assert_eq!(clean_prompt(&once), once);
    }

    #[test]
    fn system_messages_are_not_prompts() {
        let raw = br#"{"hook_event_name":"UserPromptSubmit","session_id":"s","cwd":"/x","prompt":"<task-notification> <task-id>by4</task-id> done"}"#;
        assert!(parse_hook(raw, Timestamp::UNIX_EPOCH).unwrap().is_none());
        assert!(is_system_message(
            "  <local-command-stdout>ok</local-command-stdout>"
        ));
        assert!(!is_system_message("why does <task-notification> show up?"));
        assert!(!is_system_message("<b>bold</b> please"));
    }

    #[test]
    fn image_placeholders_are_dropped() {
        assert_eq!(
            collapse_ws(&strip_image_refs(
                "[Image #56] [Image #57][Image #58] add a badge"
            )),
            "add a badge"
        );
        assert_eq!(strip_image_refs("[Image #1]"), "[image]");
        assert_eq!(
            strip_image_refs("see [Image #x] here"),
            "see [Image #x] here"
        );
    }

    #[test]
    fn recap_takes_two_sentences_of_first_paragraph() {
        let r = recap("**Done.** Tests pass! Next: UI.\n\nDetails here.").unwrap();
        assert_eq!(r, "Done. Tests pass!");
        assert_eq!(recap("   \n\n  "), None);
        assert_eq!(recap("## Summary\n\nAll good"), Some("Summary".into()));
    }

    #[test]
    fn recap_is_bounded() {
        let long = "a".repeat(1000);
        assert_eq!(recap(&long).unwrap().chars().count(), RECAP_MAX_CHARS);
    }

    #[test]
    fn unknown_notification_type_is_ignored() {
        let raw = br#"{"hook_event_name":"Notification","session_id":"s","cwd":"/x","message":"Auth ok","notification_type":"auth_success"}"#;
        assert!(parse_hook(raw, Timestamp::UNIX_EPOCH).unwrap().is_none());
    }

    #[test]
    fn missing_fields_are_errors_not_panics() {
        assert!(parse_hook(b"[1,2]", Timestamp::UNIX_EPOCH).is_err());
        assert!(parse_hook(br#"{"hook_event_name":"Stop"}"#, Timestamp::UNIX_EPOCH).is_err());
        assert!(parse_hook(b"\xff\xfe", Timestamp::UNIX_EPOCH).is_err());
    }
}
