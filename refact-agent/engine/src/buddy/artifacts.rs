use std::sync::Arc;

use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock as ARwLock;

use crate::buddy::memory_lifecycle::{
    apply_memory_lifecycle_op_status, MemoryCandidate, MemoryCandidateStatus, MemoryLifecycleOp,
    MemoryLifecyclePayload, MemoryOpStatus, MemoryOpType, MemorySource,
};
use crate::global_context::GlobalContext;

const MAX_STRUCTURED_ARTIFACTS: usize = 8;
const REPORT_MAX_CHARS: usize = 12_000;
const ARTIFACT_MAX_CHARS: usize = 4_000;

fn artifact_hash(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.len().to_string().as_bytes());
        hasher.update(b"\0");
        hasher.update(part.as_bytes());
        hasher.update(b"\0");
    }
    hex::encode(hasher.finalize())
}

fn cap_text(text: &str, max_chars: usize) -> String {
    crate::llm::safe_truncate(text.trim(), max_chars).to_string()
}

async fn project_root(gcx: Arc<ARwLock<GlobalContext>>) -> Option<std::path::PathBuf> {
    crate::files_correction::get_project_dirs(gcx)
        .await
        .into_iter()
        .next()
}

fn finalized(status: MemoryOpStatus) -> bool {
    matches!(
        status,
        MemoryOpStatus::Applied
            | MemoryOpStatus::Rejected
            | MemoryOpStatus::Failed
            | MemoryOpStatus::Skipped
    )
}

async fn enqueue_and_apply_artifact(
    gcx: Arc<ARwLock<GlobalContext>>,
    op: MemoryLifecycleOp,
) -> Result<MemoryOpStatus, String> {
    let root = project_root(gcx.clone())
        .await
        .ok_or_else(|| "buddy artifact persistence has no project root".to_string())?;
    let state = crate::buddy::storage::enqueue_memory_op(&root, op.clone()).await?;
    let Some(saved) = state.matching_op(&op).cloned() else {
        return Ok(MemoryOpStatus::Skipped);
    };
    if finalized(saved.status) {
        return Ok(saved.status);
    }
    if saved.requires_approval && saved.status != MemoryOpStatus::Approved {
        return Ok(saved.status);
    }
    let updated = apply_memory_lifecycle_op_status(gcx, &saved).await;
    let status = updated.status;
    crate::buddy::storage::enqueue_memory_op(&root, updated).await?;
    Ok(status)
}

async fn create_memory_artifact(
    gcx: Arc<ARwLock<GlobalContext>>,
    title: String,
    content: String,
    mut tags: Vec<String>,
    kind: String,
    source_id: String,
    evidence: String,
    confidence: f32,
    status: MemoryCandidateStatus,
) -> Result<MemoryOpStatus, String> {
    if content.trim().is_empty() {
        return Ok(MemoryOpStatus::Skipped);
    }
    tags.push("buddy".to_string());
    tags.push("artifact".to_string());
    let candidate = MemoryCandidate {
        candidate_id: format!("memcand_buddy_{}", artifact_hash(&[&source_id, &title])),
        source: MemorySource::Buddy,
        title,
        content,
        tags,
        kind,
        source_id: Some(source_id.clone()),
        confidence,
        status,
        ..Default::default()
    };
    let op = candidate.into_create_memory_op(
        format!("memop_buddy_{}", artifact_hash(&[&source_id])),
        evidence,
        Utc::now().to_rfc3339(),
    );
    enqueue_and_apply_artifact(gcx, op).await
}

pub async fn persist_autonomous_report_artifacts(
    gcx: Arc<ARwLock<GlobalContext>>,
    workflow_id: &str,
    title: &str,
    signal_hash: &str,
    chat_id: &str,
    report_text: &str,
) -> usize {
    let report = cap_text(report_text, REPORT_MAX_CHARS);
    if report.is_empty() {
        return 0;
    }

    let mut persisted = 0usize;
    let source_id = format!("buddy_autonomous_report:{workflow_id}:{signal_hash}");
    let content = format!(
        "# {title}\n\nSaved autonomous Buddy chat: `{chat_id}`\nWorkflow: `{workflow_id}`\nSignal: `{signal_hash}`\n\n{report}"
    );
    match create_memory_artifact(
        gcx.clone(),
        format!("{title} report"),
        content,
        vec!["autonomous".to_string(), workflow_id.to_string()],
        "artifact".to_string(),
        source_id,
        format!("Buddy autonomous workflow {workflow_id} produced saved chat {chat_id}"),
        0.80,
        MemoryCandidateStatus::Proposed,
    )
    .await
    {
        Ok(MemoryOpStatus::Applied)
        | Ok(MemoryOpStatus::Pending)
        | Ok(MemoryOpStatus::Approved) => {
            persisted += 1;
        }
        Ok(_) => {}
        Err(err) => tracing::warn!(
            "buddy: failed to persist autonomous report artifact: {}",
            err
        ),
    }

    persisted +=
        persist_structured_artifacts(gcx, workflow_id, title, signal_hash, chat_id, &report).await;
    persisted
}

#[allow(dead_code)]
pub async fn persist_humor_artifacts(
    gcx: Arc<ARwLock<GlobalContext>>,
    kind: &str,
    pulse_summary: &str,
    lines: &[String],
) -> usize {
    let clean = lines
        .iter()
        .map(|line| cap_text(line, 120))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if clean.is_empty() {
        return 0;
    }
    let joined = clean
        .iter()
        .map(|line| format!("- {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let humor_hash = artifact_hash(&[&clean.join("\n")]);
    let source_id = format!("buddy_humor:{kind}:{}", &humor_hash[..16]);
    match create_memory_artifact(
        gcx,
        format!("Buddy humor for {kind}"),
        format!("# Buddy humor for {kind}\n\nReal state: {pulse_summary}\n\n{joined}"),
        vec!["humor".to_string(), kind.to_string()],
        "humor".to_string(),
        source_id,
        format!(
            "Buddy humor generator produced {} real one-liners",
            clean.len()
        ),
        0.80,
        MemoryCandidateStatus::Proposed,
    )
    .await
    {
        Ok(MemoryOpStatus::Applied)
        | Ok(MemoryOpStatus::Pending)
        | Ok(MemoryOpStatus::Approved) => 1,
        Ok(_) => 0,
        Err(err) => {
            tracing::warn!("buddy: failed to persist humor artifact: {}", err);
            0
        }
    }
}

async fn persist_structured_artifacts(
    gcx: Arc<ARwLock<GlobalContext>>,
    workflow_id: &str,
    title: &str,
    signal_hash: &str,
    chat_id: &str,
    report_text: &str,
) -> usize {
    let Some(value) = extract_artifact_json(report_text) else {
        return 0;
    };
    let artifacts = value.get("artifacts").unwrap_or(&value);
    let mut count = 0usize;

    count += persist_create_items(
        gcx.clone(),
        artifacts.get("memories_to_add"),
        workflow_id,
        signal_hash,
        chat_id,
        "memory",
    )
    .await;
    count += persist_create_items(
        gcx.clone(),
        artifacts.get("insights"),
        workflow_id,
        signal_hash,
        chat_id,
        "insight",
    )
    .await;
    count += persist_string_items(
        gcx.clone(),
        artifacts.get("jokes"),
        workflow_id,
        signal_hash,
        chat_id,
        "joke",
        "humor",
    )
    .await;
    count += enqueue_review_ops(gcx, artifacts, workflow_id, title, signal_hash, chat_id).await;
    count
}

async fn persist_create_items(
    gcx: Arc<ARwLock<GlobalContext>>,
    value: Option<&Value>,
    workflow_id: &str,
    signal_hash: &str,
    chat_id: &str,
    default_kind: &str,
) -> usize {
    let Some(items) = value.and_then(|v| v.as_array()) else {
        return 0;
    };
    let mut count = 0usize;
    for (idx, item) in items.iter().take(MAX_STRUCTURED_ARTIFACTS).enumerate() {
        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| cap_text(s, 160))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("Buddy {default_kind} from {workflow_id}"));
        let content = item
            .get("content")
            .or_else(|| item.get("summary"))
            .and_then(|v| v.as_str())
            .map(|s| cap_text(s, ARTIFACT_MAX_CHARS))
            .unwrap_or_default();
        if content.is_empty() {
            continue;
        }
        let mut tags = item
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|values| {
                values
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        tags.push(workflow_id.to_string());
        let kind = item
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or(default_kind)
            .to_string();
        let source_id = format!(
            "buddy_structured_artifact:{workflow_id}:{signal_hash}:{chat_id}:{default_kind}:{idx}"
        );
        match create_memory_artifact(
            gcx.clone(),
            title,
            content,
            tags,
            kind,
            source_id,
            format!("Buddy autonomous workflow {workflow_id} emitted structured {default_kind}"),
            0.80,
            MemoryCandidateStatus::Proposed,
        )
        .await
        {
            Ok(MemoryOpStatus::Applied)
            | Ok(MemoryOpStatus::Pending)
            | Ok(MemoryOpStatus::Approved) => {
                count += 1;
            }
            Ok(_) => {}
            Err(err) => tracing::warn!("buddy: failed to persist structured artifact: {}", err),
        }
    }
    count
}

async fn persist_string_items(
    gcx: Arc<ARwLock<GlobalContext>>,
    value: Option<&Value>,
    workflow_id: &str,
    signal_hash: &str,
    chat_id: &str,
    item_label: &str,
    kind: &str,
) -> usize {
    let Some(items) = value.and_then(|v| v.as_array()) else {
        return 0;
    };
    let objects = items
        .iter()
        .filter_map(|item| item.as_str())
        .map(|text| serde_json::json!({"title": format!("Buddy {item_label}"), "content": text, "kind": kind, "tags": [item_label, kind]}))
        .collect::<Vec<_>>();
    persist_create_items(
        gcx,
        Some(&Value::Array(objects)),
        workflow_id,
        signal_hash,
        chat_id,
        kind,
    )
    .await
}

async fn enqueue_review_ops(
    gcx: Arc<ARwLock<GlobalContext>>,
    artifacts: &Value,
    workflow_id: &str,
    title: &str,
    signal_hash: &str,
    chat_id: &str,
) -> usize {
    let Some(root) = project_root(gcx).await else {
        return 0;
    };
    let mut count = 0usize;
    for (key, op_type) in [
        ("memory_paths_to_review", MemoryOpType::MarkReviewNeeded),
        ("memory_paths_to_archive", MemoryOpType::ArchiveCandidate),
    ] {
        let paths = artifacts
            .get(key)
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(|item| item.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if paths.is_empty() {
            continue;
        }
        let op_id = format!(
            "memop_buddy_{}",
            artifact_hash(&[workflow_id, signal_hash, chat_id, key])
        );
        let mut op = MemoryLifecycleOp::pending(
            op_id,
            MemorySource::Buddy,
            op_type,
            paths,
            format!("Buddy autonomous workflow {workflow_id} ({title}) emitted {key}"),
            0.82,
            Utc::now().to_rfc3339(),
        );
        op.requires_approval = true;
        op.payload = MemoryLifecyclePayload {
            source_id: Some(format!(
                "buddy_review_artifact:{workflow_id}:{signal_hash}:{key}"
            )),
            ..Default::default()
        };
        match crate::buddy::storage::enqueue_memory_op(&root, op).await {
            Ok(_) => count += 1,
            Err(err) => tracing::warn!("buddy: failed to enqueue structured memory op: {}", err),
        }
    }
    count
}

fn extract_artifact_json(text: &str) -> Option<Value> {
    for block in fenced_json_blocks(text) {
        if let Ok(value) = serde_json::from_str::<Value>(&block) {
            if looks_like_artifacts(&value) {
                return Some(value);
            }
        }
    }
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    let value = serde_json::from_str::<Value>(&text[start..=end]).ok()?;
    looks_like_artifacts(&value).then_some(value)
}

fn fenced_json_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("```json") {
        let after = &rest[start + "```json".len()..];
        let Some(end) = after.find("```") else {
            break;
        };
        blocks.push(after[..end].trim().to_string());
        rest = &after[end + 3..];
    }
    blocks
}

fn looks_like_artifacts(value: &Value) -> bool {
    let artifacts = value.get("artifacts").unwrap_or(value);
    artifacts.get("memories_to_add").is_some()
        || artifacts.get("insights").is_some()
        || artifacts.get("jokes").is_some()
        || artifacts.get("memory_paths_to_review").is_some()
        || artifacts.get("memory_paths_to_archive").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_fenced_artifact_json() {
        let text = "Report\n```json\n{\"artifacts\":{\"insights\":[\"Use cargo check\"]}}\n```";
        let value = extract_artifact_json(text).unwrap();
        assert!(value.get("artifacts").unwrap().get("insights").is_some());
    }

    #[test]
    fn ignores_non_artifact_json() {
        assert!(extract_artifact_json("```json\n{\"ok\":true}\n```").is_none());
    }
}
