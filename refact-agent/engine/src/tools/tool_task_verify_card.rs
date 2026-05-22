use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum};
use crate::chat::verifier::{verify_card, VerifyCardRequest};
use crate::tools::tools_description::{Tool, ToolDesc, ToolSource, ToolSourceType};

pub struct ToolTaskVerifyCard;

impl ToolTaskVerifyCard {
    pub fn new() -> Self {
        Self
    }
}

fn required_string(args: &HashMap<String, Value>, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("Missing '{}'", key))
}

async fn planner_task_id(
    ccx: &Arc<AMutex<AtCommandsContext>>,
    args: &HashMap<String, Value>,
) -> Result<String, String> {
    if let Some(task_id) = args.get("task_id").and_then(|value| value.as_str()) {
        return Ok(task_id.to_string());
    }
    let ccx_lock = ccx.lock().await;
    ccx_lock
        .task_meta
        .as_ref()
        .map(|meta| meta.task_id.clone())
        .ok_or_else(|| "Missing 'task_id' (and chat is not bound to a task)".to_string())
}

#[async_trait]
impl Tool for ToolTaskVerifyCard {
    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "task_verify_card".to_string(),
            display_name: "Task Verify Card".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: String::new(),
            },
            experimental: false,
            allow_parallel: false,
            description: "Manually re-run verifier for a completed task card and store verifier_report on the card.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "card_id": {"type": "string", "description": "Card ID to verify"},
                    "task_id": {"type": "string", "description": "Task ID (optional if in task context)"}
                },
                "required": ["card_id"]
            }),
            output_schema: None,
            annotations: None,
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let is_planner = ccx
            .lock()
            .await
            .task_meta
            .as_ref()
            .map(|meta| meta.role == "planner")
            .unwrap_or(false);
        if !is_planner {
            return Err("task_verify_card can only be called by the task planner.".to_string());
        }
        let card_id = required_string(args, "card_id")?;
        let task_id = planner_task_id(&ccx, args).await?;
        let gcx = ccx.lock().await.app.gcx.clone();
        let report = verify_card(
            gcx,
            VerifyCardRequest {
                task_id,
                card_id: card_id.clone(),
            },
        )
        .await?;
        let concerns = if report.concerns.is_empty() {
            "none".to_string()
        } else {
            report.concerns.join("\n- ")
        };
        let content = format!(
            "# Verifier Report\n\n**Card:** {}\n**Passed:** {}\n**Recommendation:** {}\n\n## Concerns\n- {}",
            card_id, report.passed, report.recommendation, concerns
        );
        Ok((
            false,
            vec![ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText(content),
                tool_calls: None,
                tool_call_id: tool_call_id.clone(),
                ..Default::default()
            })],
        ))
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }
}
