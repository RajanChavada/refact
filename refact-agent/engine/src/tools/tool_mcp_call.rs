use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::ContextEnum;
use crate::tools::tools_description::{MatchConfirmDeny, MatchConfirmDenyResult, Tool, ToolConfig, ToolDesc, ToolGroupCategory, ToolSource, ToolSourceType};
use crate::tools::tools_list::get_integration_tools;

pub struct ToolMcpCall {}

#[async_trait]
impl Tool for ToolMcpCall {
    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "mcp_call".to_string(),
            experimental: false,
            allow_parallel: false,
            description: "Execute any MCP tool by name with the given arguments. \
                Use `mcp_tool_search` first to discover the tool name and its input schema, \
                then call this with the exact arguments the schema requires."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tool_name": {
                        "type": "string",
                        "description": "Exact MCP tool name as returned by mcp_tool_search"
                    },
                    "args": {
                        "type": "object",
                        "description": "Arguments object matching the tool's input schema"
                    }
                },
                "required": ["tool_name", "args"]
            }),
            output_schema: None,
            annotations: None,
            display_name: "MCP Call".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: String::new(),
            },
        }
    }

    fn config(&self) -> Result<ToolConfig, String> {
        Ok(ToolConfig { enabled: true, allow_parallel: None })
    }

    /// Proxy confirmation/deny checks to the underlying MCP tool so that
    /// `check_tools_confirmation()` can trigger the normal pause/deny flow
    /// before `tool_execute` is ever called.
    async fn match_against_confirm_deny(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>,
    ) -> Result<crate::tools::tools_description::MatchConfirmDeny, String> {
        let tool_name = match args.get("tool_name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => return Ok(MatchConfirmDeny {
                result: MatchConfirmDenyResult::PASS,
                command: String::new(),
                rule: String::new(),
            }),
        };

        let tool_args: HashMap<String, Value> = args.get("args")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        let gcx = ccx.lock().await.global_context.clone();
        let mut integration_groups = get_integration_tools(gcx).await;

        // Move the tool out of the groups so it can be awaited safely.
        let mut found_tool: Option<Box<dyn Tool + Send>> = None;
        'outer: for group in &mut integration_groups {
            if !matches!(group.category, ToolGroupCategory::MCP) {
                continue;
            }
            if let Some(pos) = group.tools.iter().position(|t| t.tool_description().name == tool_name) {
                found_tool = Some(group.tools.remove(pos));
                break 'outer;
            }
        }

        match found_tool {
            Some(tool) => tool.match_against_confirm_deny(ccx, &tool_args).await,
            None => Ok(MatchConfirmDeny {
                result: MatchConfirmDenyResult::PASS,
                command: String::new(),
                rule: String::new(),
            }),
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let tool_name = args.get("tool_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "mcp_call: missing required argument 'tool_name'".to_string())?
            .to_string();

        let tool_args: HashMap<String, Value> = match args.get("args") {
            None => return Err("mcp_call: missing required argument 'args'".to_string()),
            Some(v) => match v.as_object() {
                None => return Err("mcp_call: argument 'args' must be an object".to_string()),
                Some(obj) => obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            },
        };

        let gcx = ccx.lock().await.global_context.clone();
        let mut integration_groups = get_integration_tools(gcx).await;

        // Find the named MCP tool and extract it (needs &mut self for tool_execute).
        let mut found_tool: Option<Box<dyn Tool + Send>> = None;
        'outer: for group in &mut integration_groups {
            if !matches!(group.category, ToolGroupCategory::MCP) {
                continue;
            }
            if let Some(pos) = group.tools.iter().position(|t| t.tool_description().name == tool_name) {
                found_tool = Some(group.tools.remove(pos));
                break 'outer;
            }
        }

        let mut tool = found_tool.ok_or_else(|| {
            format!(
                "MCP tool '{}' not found. Use mcp_tool_search to discover available tools.",
                tool_name
            )
        })?;

        if !tool.config().unwrap_or_default().enabled {
            return Err(format!("MCP tool '{}' is disabled.", tool_name));
        }

        tool.tool_execute(ccx, tool_call_id, &tool_args).await
    }
}
