use std::path::Path;
use tokio::fs;
use tracing::warn;

use super::types::BuddyConversationEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuddyWorkflowMapping {
    pub kind: &'static str,
    pub icon: &'static str,
    pub badge: Option<&'static str>,
}

pub fn workflow_id_to_mapping(id: &str) -> BuddyWorkflowMapping {
    match id {
        "buddy_humor" => BuddyWorkflowMapping {
            kind: "system",
            icon: "🎭",
            badge: Some("Humor"),
        },
        "buddy_error_detective" => BuddyWorkflowMapping {
            kind: "system",
            icon: "🕵️",
            badge: Some("Error Detective"),
        },
        "buddy_memory_gardener" => BuddyWorkflowMapping {
            kind: "system",
            icon: "🌿",
            badge: Some("Memory"),
        },
        "buddy_knowledge_conflict_resolver" => BuddyWorkflowMapping {
            kind: "system",
            icon: "🧩",
            badge: Some("Knowledge"),
        },
        "buddy_behavior_learner" => BuddyWorkflowMapping {
            kind: "system",
            icon: "🧭",
            badge: Some("Preferences"),
        },
        "buddy_user_habit_coach" => BuddyWorkflowMapping {
            kind: "system",
            icon: "🏃",
            badge: Some("Habits"),
        },
        "buddy_model_cost_optimizer" => BuddyWorkflowMapping {
            kind: "system",
            icon: "💸",
            badge: Some("Model/Cost"),
        },
        "buddy_dependency_radar" => BuddyWorkflowMapping {
            kind: "system",
            icon: "📦",
            badge: Some("Dependencies"),
        },
        "buddy_docs_gardener" => BuddyWorkflowMapping {
            kind: "system",
            icon: "📚",
            badge: Some("Docs"),
        },
        "buddy_setup_coach" => BuddyWorkflowMapping {
            kind: "system",
            icon: "🧰",
            badge: Some("Setup"),
        },
        "buddy_security_whisperer" => BuddyWorkflowMapping {
            kind: "system",
            icon: "🛡️",
            badge: Some("Security"),
        },
        "buddy_architecture_drift_watcher" => BuddyWorkflowMapping {
            kind: "system",
            icon: "🏗️",
            badge: Some("Architecture"),
        },
        "commit_message" => BuddyWorkflowMapping {
            kind: "workflow",
            icon: "🔄",
            badge: Some("Commit Msg"),
        },
        "follow_up" => BuddyWorkflowMapping {
            kind: "workflow",
            icon: "💡",
            badge: Some("Follow-up"),
        },
        "compress_trajectory" => BuddyWorkflowMapping {
            kind: "system",
            icon: "🤖",
            badge: Some("Compress"),
        },
        "memo_extraction" => BuddyWorkflowMapping {
            kind: "system",
            icon: "🧠",
            badge: Some("Memo"),
        },
        "kg_enrich" | "kg_deprecate" => BuddyWorkflowMapping {
            kind: "system",
            icon: "📚",
            badge: Some("Knowledge"),
        },
        _ => BuddyWorkflowMapping {
            kind: "workflow",
            icon: "🔄",
            badge: None,
        },
    }
}

pub async fn list_all_buddy_conversations(
    project_root: &Path,
    kind_filter: Option<Vec<String>>,
) -> Vec<BuddyConversationEntry> {
    let mut entries = Vec::new();

    let conv_dir = project_root.join(".refact/buddy/chats/conversations");
    if let Ok(mut rd) = fs::read_dir(&conv_dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let path = entry.path();
            if !path.extension().map(|e| e == "json").unwrap_or(false) {
                continue;
            }
            let content = match fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(_) => continue,
            };
            let val = match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(v) => v,
                Err(_) => {
                    warn!("buddy: skipping malformed conversation file: {:?}", path);
                    continue;
                }
            };
            let id = val
                .get("chat_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if id.is_empty() {
                warn!("buddy: conversation file missing chat_id: {:?}", path);
                continue;
            }
            let kind = val
                .get("kind")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    val.get("buddy_meta")
                        .and_then(|meta| meta.get("buddy_chat_kind"))
                        .and_then(|v| v.as_str())
                })
                .unwrap_or("chat")
                .to_string();
            let title = val
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string();
            let created = val
                .get("created_at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let updated = val
                .get("last_message_at")
                .and_then(|v| v.as_str())
                .unwrap_or(&created)
                .to_string();
            let msgs = val
                .get("messages")
                .and_then(|v| v.as_array())
                .map(|a| a.len() as u32)
                .unwrap_or(0);
            let badge = val
                .get("badge")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    val.get("buddy_meta")
                        .and_then(|meta| meta.get("workflow_id"))
                        .and_then(|workflow_id| workflow_id.as_str())
                        .and_then(|workflow_id| workflow_id_to_mapping(workflow_id).badge)
                        .map(|s| s.to_string())
                });
            let icon = match kind.as_str() {
                "setup" => "⚙️".to_string(),
                "analysis" => "🔍".to_string(),
                "system" => val
                    .get("buddy_meta")
                    .and_then(|meta| meta.get("workflow_id"))
                    .and_then(|workflow_id| workflow_id.as_str())
                    .map(|workflow_id| workflow_id_to_mapping(workflow_id).icon.to_string())
                    .unwrap_or_else(|| "🤖".to_string()),
                _ => "💬".to_string(),
            };
            entries.push(BuddyConversationEntry {
                id,
                kind,
                title,
                created_at: created,
                updated_at: updated,
                status: "active".to_string(),
                message_count: msgs,
                icon,
                badge,
            });
        }
    }

    let wf_dir = project_root.join(".refact/buddy/chats/workflows");
    if let Ok(mut rd) = fs::read_dir(&wf_dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let path = entry.path();
            if !path.extension().map(|e| e == "json").unwrap_or(false) {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let content = match fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(_) => continue,
            };
            let val = match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(v) => v,
                Err(_) => {
                    warn!("buddy: skipping malformed workflow file: {:?}", path);
                    continue;
                }
            };
            let mapping = workflow_id_to_mapping(&stem);
            let entry_count = val
                .get("entries")
                .and_then(|v| v.as_array())
                .map(|a| a.len() as u32)
                .unwrap_or(0);
            let last_ts = val
                .get("entries")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.last())
                .and_then(|e| e.get("timestamp"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            entries.push(BuddyConversationEntry {
                id: stem.clone(),
                kind: mapping.kind.to_string(),
                title: format!(
                    "{}{}",
                    stem.replace('_', " "),
                    mapping
                        .badge
                        .map(|b| format!(" ({})", b))
                        .unwrap_or_default()
                ),
                created_at: last_ts.clone(),
                updated_at: last_ts,
                status: "completed".to_string(),
                message_count: entry_count,
                icon: mapping.icon.to_string(),
                badge: mapping.badge.map(|s| s.to_string()),
            });
        }
    }

    if let Some(filter) = &kind_filter {
        entries.retain(|e| filter.iter().any(|f| f == &e.kind));
    }

    entries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autonomous_workflow_ids_have_system_mappings() {
        let expected = [
            ("buddy_error_detective", "🕵️", "Error Detective"),
            ("buddy_memory_gardener", "🌿", "Memory"),
            ("buddy_knowledge_conflict_resolver", "🧩", "Knowledge"),
            ("buddy_behavior_learner", "🧭", "Preferences"),
            ("buddy_user_habit_coach", "🏃", "Habits"),
            ("buddy_model_cost_optimizer", "💸", "Model/Cost"),
            ("buddy_dependency_radar", "📦", "Dependencies"),
            ("buddy_docs_gardener", "📚", "Docs"),
            ("buddy_setup_coach", "🧰", "Setup"),
            ("buddy_security_whisperer", "🛡️", "Security"),
            ("buddy_architecture_drift_watcher", "🏗️", "Architecture"),
        ];

        for (workflow_id, icon, badge) in expected {
            let mapping = workflow_id_to_mapping(workflow_id);
            assert_eq!(mapping.kind, "system");
            assert_eq!(mapping.icon, icon);
            assert_eq!(mapping.badge, Some(badge));
        }
    }

    #[test]
    fn unknown_workflow_mapping_remains_workflow_fallback() {
        let mapping = workflow_id_to_mapping("custom_workflow");

        assert_eq!(mapping.kind, "workflow");
        assert_eq!(mapping.icon, "🔄");
        assert_eq!(mapping.badge, None);
    }
}
