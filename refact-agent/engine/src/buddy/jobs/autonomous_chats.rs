#![allow(dead_code)]

use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock as ARwLock;
use uuid::Uuid;

use crate::buddy::actor::{redact_sensitive, validate_workflow_id};
use crate::buddy::scheduler::BuddyJobContext;
use crate::buddy::types::BuddyThreadMeta;
use crate::call_validation::ChatMessage;
use crate::global_context::GlobalContext;

pub const AUTONOMOUS_BUDDY_CHAT_SUBAGENT: &str = "buddy_autonomous_chat";
pub const AUTONOMOUS_PROMPT_CAP_CHARS: usize = 8_000;
pub const AUTONOMOUS_EVIDENCE_CAP_CHARS: usize = 24_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomousBuddyChatSpec {
    pub workflow_id: String,
    pub title: String,
    pub prompt: String,
    pub evidence: String,
    pub signal_hash: String,
    pub icon: String,
    pub badge: String,
    pub priority: String,
}

impl AutonomousBuddyChatSpec {
    pub fn new(
        workflow_id: impl Into<String>,
        title: impl Into<String>,
        prompt: impl Into<String>,
        evidence: impl Into<String>,
    ) -> Self {
        let workflow_id = workflow_id.into();
        let title = title.into();
        let prompt = prompt.into();
        let evidence = evidence.into();
        let signal_hash = signal_hash([
            workflow_id.as_str(),
            title.as_str(),
            prompt.as_str(),
            evidence.as_str(),
        ]);
        Self {
            workflow_id,
            title,
            prompt,
            evidence,
            signal_hash,
            icon: String::new(),
            badge: String::new(),
            priority: "normal".to_string(),
        }
    }

    pub fn with_display(
        mut self,
        icon: impl Into<String>,
        badge: impl Into<String>,
        priority: impl Into<String>,
    ) -> Self {
        self.icon = icon.into();
        self.badge = badge.into();
        self.priority = priority.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousLastResult {
    pub signal_hash: String,
    pub chat_id: String,
    pub completed_at: String,
}

impl AutonomousLastResult {
    pub fn new(signal_hash: impl Into<String>, chat_id: impl Into<String>) -> Self {
        Self {
            signal_hash: signal_hash.into(),
            chat_id: chat_id.into(),
            completed_at: Utc::now().to_rfc3339(),
        }
    }
}

pub fn signal_hash<I, S>(parts: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut hasher = Sha256::new();
    for part in parts {
        let text = part.as_ref();
        hasher.update(text.len().to_string().as_bytes());
        hasher.update(b"\0");
        hasher.update(text.as_bytes());
        hasher.update(b"\0");
    }
    hex::encode(hasher.finalize())
}

pub fn parse_last_autonomous_result(raw: Option<&str>) -> Option<AutonomousLastResult> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    let parsed = serde_json::from_str::<AutonomousLastResult>(raw).ok()?;
    if parsed.signal_hash.is_empty() || parsed.chat_id.is_empty() || parsed.completed_at.is_empty()
    {
        return None;
    }
    Some(parsed)
}

pub fn serialize_last_autonomous_result(result: &AutonomousLastResult) -> String {
    serde_json::json!({
        "signal_hash": result.signal_hash,
        "chat_id": result.chat_id,
        "completed_at": result.completed_at,
    })
    .to_string()
}

pub fn same_signal(ctx: &BuddyJobContext, hash: &str) -> bool {
    parse_last_autonomous_result(ctx.job_state.last_result.as_deref())
        .map(|last| last.signal_hash == hash)
        .unwrap_or(false)
}

pub fn redact_and_cap_prompt(text: &str) -> String {
    redact_and_cap_text(text, AUTONOMOUS_PROMPT_CAP_CHARS)
}

pub fn redact_and_cap_evidence(text: &str) -> String {
    redact_and_cap_text(text, AUTONOMOUS_EVIDENCE_CAP_CHARS)
}

pub fn redact_and_cap_text(text: &str, max_chars: usize) -> String {
    let redacted = redact_sensitive(text);
    if max_chars == 0 {
        return String::new();
    }
    if redacted.len() <= max_chars {
        return redacted;
    }
    const MARKER: &str = "\n...[truncated]";
    if max_chars <= MARKER.len() {
        return crate::llm::safe_truncate(MARKER, max_chars).to_string();
    }
    let keep = max_chars - MARKER.len();
    let prefix = crate::llm::safe_truncate(&redacted, keep)
        .trim_end()
        .to_string();
    format!("{}{}", prefix, MARKER)
}

pub async fn run_autonomous_buddy_chat(
    gcx: Arc<ARwLock<GlobalContext>>,
    spec: AutonomousBuddyChatSpec,
) -> Result<String, String> {
    if !validate_workflow_id(&spec.workflow_id) {
        return Err(format!(
            "invalid autonomous buddy workflow id: {}",
            spec.workflow_id
        ));
    }

    let (messages, max_steps) = build_autonomous_messages(gcx.clone(), &spec).await?;

    let mut config = crate::subchat::resolve_subchat_config(
        gcx.clone(),
        AUTONOMOUS_BUDDY_CHAT_SUBAGENT,
        true,
        Some(format!("buddy-{}-{}", spec.workflow_id, Uuid::new_v4())),
        Some(spec.title.clone()),
        None,
        None,
        None,
        Some(vec![]),
        max_steps,
        false,
        None,
        "buddy".to_string(),
    )
    .await?;

    config.mode = "buddy".to_string();
    config.buddy_meta = Some(BuddyThreadMeta {
        is_buddy_chat: true,
        buddy_chat_kind: "system".to_string(),
        workflow_id: Some(spec.workflow_id.clone()),
    });

    let result = crate::subchat::run_subchat(gcx, messages, config).await?;
    result
        .chat_id
        .ok_or_else(|| "autonomous buddy chat did not return a chat_id".to_string())
}

async fn build_autonomous_messages(
    gcx: Arc<ARwLock<GlobalContext>>,
    spec: &AutonomousBuddyChatSpec,
) -> Result<(Vec<ChatMessage>, usize), String> {
    let subagent_config = crate::yaml_configs::customization_registry::get_subagent_config(
        gcx,
        AUTONOMOUS_BUDDY_CHAT_SUBAGENT,
        None,
    )
    .await
    .ok_or_else(|| {
        format!(
            "subagent config '{}' not found",
            AUTONOMOUS_BUDDY_CHAT_SUBAGENT
        )
    })?;

    let system_prompt = subagent_config.messages.system_prompt.ok_or_else(|| {
        format!(
            "messages.system_prompt not defined for subagent '{}'",
            AUTONOMOUS_BUDDY_CHAT_SUBAGENT
        )
    })?;
    let user_template = subagent_config.messages.user_template.ok_or_else(|| {
        format!(
            "messages.user_template not defined for subagent '{}'",
            AUTONOMOUS_BUDDY_CHAT_SUBAGENT
        )
    })?;

    let max_steps = subagent_config
        .subchat
        .max_steps
        .unwrap_or(1)
        .max(1)
        .min(10);
    let user_prompt = render_autonomous_template(&user_template, spec);
    let messages = vec![
        ChatMessage::new("system".to_string(), system_prompt),
        ChatMessage::new("user".to_string(), user_prompt),
    ];
    Ok((messages, max_steps))
}

fn render_autonomous_template(template: &str, spec: &AutonomousBuddyChatSpec) -> String {
    let prompt = redact_and_cap_prompt(&spec.prompt);
    let evidence = redact_and_cap_evidence(&spec.evidence);
    let replacements = [
        ("{{workflow_id}}", spec.workflow_id.as_str()),
        ("{{title}}", spec.title.as_str()),
        ("{{signal_hash}}", spec.signal_hash.as_str()),
        ("{{icon}}", spec.icon.as_str()),
        ("{{badge}}", spec.badge.as_str()),
        ("{{priority}}", spec.priority.as_str()),
        ("{{prompt}}", prompt.as_str()),
        ("{{evidence}}", evidence.as_str()),
    ];
    let mut rendered = template.to_string();
    for (needle, value) in replacements {
        rendered = rendered.replace(needle, value);
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buddy::scheduler::BuddyJobContext;
    use crate::buddy::settings::BuddySettings;
    use crate::buddy::types::{BuddyJobState, BuddyOnboarding, BuddyPetState, BuddyPulse};

    fn context_with_last_result(last_result: Option<String>) -> BuddyJobContext {
        BuddyJobContext {
            identity_name: "Pixel".to_string(),
            onboarding: BuddyOnboarding::default(),
            recent_diagnostics: vec![],
            project_root: std::path::PathBuf::from("/tmp/project"),
            job_state: BuddyJobState {
                last_result,
                ..Default::default()
            },
            total_workflow_runs: 0,
            suggestion_state: vec![],
            pet: BuddyPetState::default(),
            active_quest: None,
            settings: BuddySettings::default(),
            pulse: BuddyPulse::default(),
            facts: vec![],
        }
    }

    #[test]
    fn signal_hash_is_stable_and_changes_with_signal() {
        let first = signal_hash(["buddy_error_detective", "a", "b"]);
        let second = signal_hash(["buddy_error_detective", "a", "b"]);
        let changed = signal_hash(["buddy_error_detective", "a", "c"]);
        let boundary_a = signal_hash(["ab", "c"]);
        let boundary_b = signal_hash(["a", "bc"]);

        assert_eq!(first, second);
        assert_ne!(first, changed);
        assert_ne!(boundary_a, boundary_b);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn last_result_json_round_trips_and_malformed_values_fallback() {
        let result = AutonomousLastResult {
            signal_hash: "hash-a".to_string(),
            chat_id: "chat-a".to_string(),
            completed_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let serialized = serialize_last_autonomous_result(&result);

        assert_eq!(
            parse_last_autonomous_result(Some(&serialized)),
            Some(result)
        );
        assert_eq!(parse_last_autonomous_result(Some("legacy-value")), None);
        assert_eq!(parse_last_autonomous_result(Some("{")), None);
        assert_eq!(parse_last_autonomous_result(Some("{}")), None);
        assert_eq!(parse_last_autonomous_result(None), None);
    }

    #[test]
    fn same_signal_uses_parsed_last_result() {
        let result = AutonomousLastResult {
            signal_hash: "same".to_string(),
            chat_id: "chat".to_string(),
            completed_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let ctx = context_with_last_result(Some(serialize_last_autonomous_result(&result)));
        let malformed_ctx = context_with_last_result(Some("same".to_string()));

        assert!(same_signal(&ctx, "same"));
        assert!(!same_signal(&ctx, "different"));
        assert!(!same_signal(&malformed_ctx, "same"));
    }

    #[test]
    fn redaction_and_capping_remove_obvious_raw_secrets() {
        let raw = "Bearer abcdef12345 password=plainsecret sk-abcdef123456 ghp_abcdef1234567890";
        let redacted = redact_and_cap_text(raw, 512);
        let capped = redact_and_cap_text(&format!("{} {}", raw, "x".repeat(256)), 64);

        assert!(!redacted.contains("abcdef12345"));
        assert!(!redacted.contains("plainsecret"));
        assert!(!redacted.contains("sk-abcdef123456"));
        assert!(!redacted.contains("ghp_abcdef1234567890"));
        assert!(redacted.contains("[REDACTED"));
        assert!(capped.len() <= 64);
        assert!(!capped.contains("plainsecret"));
    }
}
