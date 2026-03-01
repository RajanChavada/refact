use std::collections::HashMap;
use std::sync::Arc;
use glob::Pattern;
use serde_json::{Value, json};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatUsage, ContextEnum};
use crate::custom_error::MapErrToString;
use crate::integrations::integr_abstract::IntegrationConfirmation;

pub fn command_should_be_confirmed_by_user(
    command: &String,
    commands_need_confirmation_rules: &Vec<String>,
) -> (bool, String) {
    if let Some(rule) = commands_need_confirmation_rules.iter().find(|glob| {
        let pattern = Pattern::new(glob).unwrap();
        pattern.matches(&command)
    }) {
        return (true, rule.clone());
    }
    (false, "".to_string())
}

pub fn command_should_be_denied(
    command: &String,
    commands_deny_rules: &Vec<String>,
) -> (bool, String) {
    if let Some(rule) = commands_deny_rules.iter().find(|glob| {
        let pattern = Pattern::new(glob).unwrap();
        pattern.matches(&command)
    }) {
        return (true, rule.clone());
    }
    (false, "".to_string())
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatchConfirmDenyResult {
    PASS,
    CONFIRMATION,
    DENY,
}

#[derive(Clone, Debug)]
pub struct MatchConfirmDeny {
    pub result: MatchConfirmDenyResult,
    pub command: String,
    pub rule: String,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ToolGroupCategory {
    Builtin,
    Integration,
    MCP,
    ConfigSubagent,
}

pub struct ToolGroup {
    pub name: String,
    pub description: String,
    pub category: ToolGroupCategory,
    pub tools: Vec<Box<dyn Tool + Send>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ToolSourceType {
    Builtin,
    Integration,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolSource {
    pub source_type: ToolSourceType,
    pub config_path: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolDesc {
    pub name: String,
    #[serde(default)]
    pub experimental: bool,
    #[serde(default)]
    pub allow_parallel: bool,
    pub description: String,
    /// Full JSON Schema for tool input parameters.
    /// Must be `{"type": "object", "properties": {...}, "required": [...]}`.
    /// For tools with no parameters, use `{"type": "object", "properties": {}}`.
    pub input_schema: serde_json::Value,
    /// Optional JSON Schema for structured output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    /// MCP-style tool annotations (readOnlyHint, destructiveHint, idempotentHint, openWorldHint, title).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotations: Option<serde_json::Value>,
    pub display_name: String,
    pub source: ToolSource,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct ToolConfig {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_parallel: Option<bool>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        ToolConfig { enabled: true, allow_parallel: None }
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String>;

    fn tool_description(&self) -> ToolDesc;

    async fn match_against_confirm_deny(
        &self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        args: &HashMap<String, Value>,
    ) -> Result<MatchConfirmDeny, String> {
        let command_to_match = self
            .command_to_match_against_confirm_deny(ccx.clone(), &args)
            .await
            .map_err(|e| format!("Error getting tool command to match: {}", e))?;

        if !command_to_match.is_empty() {
            if let Some(rules) = &self.confirm_deny_rules() {
                tracing::info!(
                    "confirmation: match {:?} against {:?}",
                    command_to_match,
                    rules
                );
                let (is_denied, deny_rule) =
                    command_should_be_denied(&command_to_match, &rules.deny);
                if is_denied {
                    return Ok(MatchConfirmDeny {
                        result: MatchConfirmDenyResult::DENY,
                        command: command_to_match.clone(),
                        rule: deny_rule.clone(),
                    });
                }
                let (needs_confirmation, confirmation_rule) =
                    command_should_be_confirmed_by_user(&command_to_match, &rules.ask_user);
                if needs_confirmation {
                    return Ok(MatchConfirmDeny {
                        result: MatchConfirmDenyResult::CONFIRMATION,
                        command: command_to_match.clone(),
                        rule: confirmation_rule.clone(),
                    });
                }
            } else {
                tracing::error!("No confirmation info available for {:?}", command_to_match);
            }
        }
        Ok(MatchConfirmDeny {
            result: MatchConfirmDenyResult::PASS,
            command: command_to_match.clone(),
            rule: "".to_string(),
        })
    }

    async fn command_to_match_against_confirm_deny(
        &self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        _args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        Ok("".to_string())
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        None
    }

    fn has_config_path(&self) -> Option<String> {
        return None;
    }

    fn config(&self) -> Result<ToolConfig, String> {
        let tool_desc = self.tool_description();

        let tool_name = tool_desc.name;
        let config_path = tool_desc.source.config_path;

        let config = std::fs::read_to_string(config_path)
            .map_err(|e| format!("Error reading config file: {}", e))?;

        let config: serde_yaml::Value = serde_yaml::from_str(&config)
            .map_err(|e| format!("Error parsing config file: {}", e))?;

        let config = config.get("tools").and_then(|tools| tools.get(&tool_name));

        match config {
            None => Ok(ToolConfig::default()),
            Some(config) => {
                let config: ToolConfig = serde_yaml::from_value(config.clone()).unwrap_or_default();
                Ok(config)
            }
        }
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    } // "ast", "vecdb"

    #[allow(dead_code)] // Trait method for future usage tracking
    fn usage(&mut self) -> &mut Option<ChatUsage> {
        static mut DEFAULT_USAGE: Option<ChatUsage> = None;
        #[allow(static_mut_refs)]
        unsafe {
            &mut DEFAULT_USAGE
        }
    }
}

pub async fn set_tool_config(
    config_path: String,
    tool_name: String,
    new_config: ToolConfig,
) -> Result<(), String> {
    let config_file = tokio::fs::read_to_string(&config_path)
        .await
        .map_err(|e| format!("Error reading config file: {}", e))?;

    let mut config: serde_yaml::Mapping = serde_yaml::from_str(&config_file)
        .map_err(|e| format!("Error parsing config file: {}", e))?;

    let tools: &mut serde_yaml::Mapping = match config
        .get_mut("tools")
        .and_then(|tools| tools.as_mapping_mut())
    {
        Some(tools) => tools,
        None => {
            config.insert(
                serde_yaml::Value::String("tools".to_string()),
                serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
            );
            config
                .get_mut("tools")
                .expect("tools was just inserted")
                .as_mapping_mut()
                .expect("tools is a mapping, it was just inserted")
        }
    };

    tools.insert(
        serde_yaml::Value::String(tool_name),
        serde_yaml::to_value(new_config)
            .map_err_with_prefix("ToolConfig should always be serializable.")?,
    );

    tokio::fs::write(config_path, serde_yaml::to_string(&config).unwrap())
        .await
        .map_err(|e| format!("Error writing config file: {}", e))?;

    Ok(())
}

/// Helper to build a simple input schema from flat parameter definitions.
/// Useful for builtin tools that have simple string/boolean/integer params.
pub fn json_schema_from_params(params: &[(&str, &str, &str)], required: &[&str]) -> Value {
    let mut properties = serde_json::Map::new();
    for (name, param_type, description) in params {
        properties.insert(name.to_string(), json!({
            "type": param_type,
            "description": description
        }));
    }
    json!({
        "type": "object",
        "properties": properties,
        "required": required
    })
}

pub fn make_openai_tool_value(
    name: String,
    description: String,
    input_schema: Value,
    strict: bool,
) -> Value {
    let mut parameters_schema = input_schema;
    if strict {
        if parameters_schema.get("type") == Some(&json!("object")) {
            if parameters_schema.get("additionalProperties").is_none() {
                parameters_schema["additionalProperties"] = json!(false);
            }
        }
    }
    let mut function_obj = json!({
        "name": name,
        "description": description,
        "parameters": parameters_schema
    });
    if strict {
        function_obj["strict"] = json!(true);
    }
    json!({
        "type": "function",
        "function": function_obj
    })
}

impl ToolDesc {
    pub fn into_openai_style(self, strict: bool) -> Value {
        make_openai_tool_value(self.name, self.description, self.input_schema, strict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_schema_from_params_basic() {
        let schema = json_schema_from_params(
            &[
                ("path", "string", "File path"),
                ("content", "string", "File content"),
            ],
            &["path"],
        );
        assert_eq!(schema["type"], json!("object"));
        assert_eq!(schema["properties"]["path"]["type"], json!("string"));
        assert_eq!(schema["properties"]["path"]["description"], json!("File path"));
        assert_eq!(schema["properties"]["content"]["type"], json!("string"));
        assert_eq!(schema["required"], json!(["path"]));
    }

    #[test]
    fn test_json_schema_from_params_no_params() {
        let schema = json_schema_from_params(&[], &[]);
        assert_eq!(schema["type"], json!("object"));
        assert_eq!(schema["properties"], json!({}));
        assert_eq!(schema["required"], json!([]));
    }

    #[test]
    fn test_make_openai_tool_value_not_strict() {
        let schema = json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search query"}
            },
            "required": ["query"]
        });
        let result = make_openai_tool_value(
            "search".to_string(),
            "Search the web".to_string(),
            schema,
            false,
        );
        assert_eq!(result["type"], json!("function"));
        assert_eq!(result["function"]["name"], json!("search"));
        assert_eq!(result["function"]["description"], json!("Search the web"));
        assert_eq!(result["function"]["parameters"]["type"], json!("object"));
        assert!(result["function"]["strict"].is_null());
        assert!(result["function"]["parameters"]["additionalProperties"].is_null());
    }

    #[test]
    fn test_make_openai_tool_value_strict() {
        let schema = json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search query"}
            },
            "required": ["query"]
        });
        let result = make_openai_tool_value(
            "search".to_string(),
            "Search the web".to_string(),
            schema,
            true,
        );
        assert_eq!(result["function"]["strict"], json!(true));
        assert_eq!(result["function"]["parameters"]["additionalProperties"], json!(false));
    }

    #[test]
    fn test_make_openai_tool_value_strict_preserves_existing_additional_properties() {
        let schema = json!({
            "type": "object",
            "properties": {},
            "additionalProperties": true
        });
        let result = make_openai_tool_value(
            "tool".to_string(),
            "A tool".to_string(),
            schema,
            true,
        );
        assert_eq!(result["function"]["parameters"]["additionalProperties"], json!(true));
    }

    #[test]
    fn test_make_openai_tool_value_complex_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "List of items"
                },
                "config": {
                    "type": "object",
                    "properties": {
                        "verbose": {"type": "boolean"}
                    }
                },
                "mode": {
                    "type": "string",
                    "enum": ["fast", "slow"]
                }
            },
            "required": ["items"]
        });
        let result = make_openai_tool_value(
            "process".to_string(),
            "Process items".to_string(),
            schema,
            false,
        );
        assert_eq!(result["function"]["parameters"]["properties"]["items"]["type"], json!("array"));
        assert_eq!(result["function"]["parameters"]["properties"]["mode"]["enum"], json!(["fast", "slow"]));
    }

    #[test]
    fn test_into_openai_style_roundtrip() {
        let input_schema = json!({
            "type": "object",
            "properties": {
                "filename": {"type": "string", "description": "The filename"}
            },
            "required": ["filename"]
        });
        let desc = ToolDesc {
            name: "cat".to_string(),
            experimental: false,
            allow_parallel: true,
            description: "Read a file".to_string(),
            input_schema: input_schema.clone(),
            output_schema: None,
            annotations: None,
            display_name: "Cat".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: "".to_string(),
            },
        };
        let result = desc.into_openai_style(false);
        assert_eq!(result["function"]["name"], json!("cat"));
        assert_eq!(result["function"]["parameters"]["properties"]["filename"]["type"], json!("string"));
    }
}
