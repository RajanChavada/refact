use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum};
use crate::tools::task_tool_helpers::require_bound_planner_task;
use crate::tools::tool_task_check_agents::{
    agent_status_input_schema, format_agent_statuses, get_agent_statuses,
    has_active_agent_statuses, parse_agent_status_query,
};
use crate::tools::tools_description::{Tool, ToolDesc, ToolSource, ToolSourceType};

pub struct ToolTaskWaitForAgents;

impl ToolTaskWaitForAgents {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ToolTaskWaitForAgents {
    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "task_wait_for_agents".to_string(),
            display_name: "Task Wait For Agents".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: String::new(),
            },
            experimental: false,
            allow_parallel: false,
            description: "Check spawned task agents using the same compact status view as task_check_agents, then stop the planner turn so it waits for agent completion messages.".to_string(),
            input_schema: agent_status_input_schema(),
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
        let ccx_lock = ccx.lock().await;

        let is_planner = ccx_lock
            .task_meta
            .as_ref()
            .map(|m| m.role == "planner")
            .unwrap_or(false);

        if !is_planner {
            return Err(
                "task_wait_for_agents can only be called by the task planner. \
                 Switch to the planner chat to check agent status."
                    .to_string(),
            );
        }

        drop(ccx_lock);

        let query = parse_agent_status_query(args)?;
        let task_id = require_bound_planner_task(&ccx, args).await?;
        let (gcx, chat_facade) = {
            let ccx_lock = ccx.lock().await;
            (ccx_lock.app.gcx.clone(), ccx_lock.app.chat.facade.clone())
        };

        let statuses = get_agent_statuses(gcx, chat_facade, &task_id).await?;
        let mut result = format_agent_statuses(&statuses, &query)?;

        if statuses.is_empty() {
            result.push_str("\nNo agents are currently running.\n");
        } else if has_active_agent_statuses(&statuses) {
            result.push_str("\n⏳ **Agents are still working.** Do not check again, wait for the completion message to arrive.\n");
        } else {
            result.push_str("\nNo agents are currently running.\n");
        }

        {
            let ccx_lock = ccx.lock().await;
            ccx_lock
                .abort_flag
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }

        Ok((
            false,
            vec![ContextEnum::ChatMessage(ChatMessage {
                role: "tool".to_string(),
                content: ChatContent::SimpleText(result),
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
