//! Claude Code OAuth compatibility layer.
//! When users authenticate via Claude Code OAuth, the API requires specific
//! headers, user-agent, system prompt prefix, and tool name prefixing.

use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{json, Value};

pub const OAUTH_BETA_FLAG: &str = "oauth-2025-04-20";
pub const USER_AGENT: &str = "claude-cli/2.1.2 (external, cli)";
pub const SYSTEM_PREFIX: &str = "You are Claude Code, Anthropic's official CLI for Claude.";
pub const MCP_TOOL_PREFIX: &str = "mcp_";

pub fn is_claude_code_oauth(auth_token: &str) -> bool {
    !auth_token.is_empty()
}

pub fn apply_oauth_headers(headers: &mut HeaderMap, auth_token: &str) -> Result<(), String> {
    headers.insert(
        "authorization",
        HeaderValue::from_str(&format!("Bearer {}", auth_token))
            .map_err(|e| format!("invalid auth_token: {e}"))?,
    );
    headers.insert("user-agent", HeaderValue::from_static(USER_AGENT));
    Ok(())
}

pub fn build_oauth_url(endpoint: &str) -> String {
    let sep = if endpoint.contains('?') { "&" } else { "?" };
    format!("{}{}beta=true", endpoint, sep)
}

pub fn prepend_system(system: Value) -> Value {
    match system {
        Value::String(text) => {
            if text.trim().is_empty() {
                json!(SYSTEM_PREFIX)
            } else {
                json!([
                    {"type": "text", "text": SYSTEM_PREFIX},
                    {"type": "text", "text": text}
                ])
            }
        }
        Value::Array(blocks) => {
            let mut new_blocks = vec![json!({"type": "text", "text": SYSTEM_PREFIX})];
            new_blocks.extend(blocks);

            if let Some(second_text) = new_blocks
                .get(1)
                .and_then(|v| {
                    v.get("type")
                        .and_then(|t| t.as_str())
                        .filter(|&t| t == "text")
                        .and_then(|_| v.get("text").and_then(|t| t.as_str()))
                })
            {
                if !second_text.starts_with(SYSTEM_PREFIX) {
                    new_blocks[1] = json!({
                        "type": "text",
                        "text": format!("{}\n\n{}", SYSTEM_PREFIX, second_text),
                    });
                }
            }
            json!(new_blocks)
        }
        _ => json!(SYSTEM_PREFIX),
    }
}

/// Prefix all tool names in an Anthropic tools array with the given prefix.
/// Required for Claude Code OAuth: Anthropic's server expects tools to be
/// prefixed with "mcp_" when using subscription-based OAuth tokens.
pub fn prefix_tool_names(tools: &mut Value, prefix: &str) {
    if let Some(arr) = tools.as_array_mut() {
        for tool in arr {
            if let Some(name) = tool.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()) {
                if !name.starts_with(prefix) {
                    tool["name"] = json!(format!("{}{}", prefix, name));
                }
            }
        }
    }
}

/// Prefix tool_use block names in message content with the given prefix.
/// Required for Claude Code OAuth when replaying historical messages.
pub fn prefix_tool_use_in_messages(messages: &mut Value, prefix: &str) {
    if let Some(msgs) = messages.as_array_mut() {
        for msg in msgs {
            if let Some(content) = msg.get_mut("content").and_then(|c| c.as_array_mut()) {
                for block in content {
                    if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                        if let Some(name) = block.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()) {
                            if !name.starts_with(prefix) {
                                block["name"] = json!(format!("{}{}", prefix, name));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_claude_code_oauth_detection() {
        assert!(is_claude_code_oauth("some-oauth-token"));
        assert!(!is_claude_code_oauth(""));
    }

    #[test]
    fn test_prepend_system_keeps_prefix_as_standalone_block() {
        let system = json!("Be helpful");
        let prefixed = prepend_system(system);
        assert!(prefixed.is_array());
        let arr = prefixed.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["type"], "text");
        assert_eq!(arr[0]["text"], SYSTEM_PREFIX);
        assert_eq!(arr[1]["type"], "text");
        assert_eq!(arr[1]["text"], "Be helpful");

        let system2 = json!([
            {"type": "text", "text": "Be helpful"},
            {"type": "text", "text": "Also be brief"}
        ]);
        let prefixed2 = prepend_system(system2);
        let arr2 = prefixed2.as_array().unwrap();
        assert_eq!(arr2[0]["text"], SYSTEM_PREFIX);
        assert_eq!(arr2[1]["text"], "You are Claude Code, Anthropic's official CLI for Claude.\n\nBe helpful");
        assert_eq!(arr2[2]["text"], "Also be brief");
    }

    #[test]
    fn test_build_oauth_url_no_existing_params() {
        let url = build_oauth_url("https://api.anthropic.com/v1/messages");
        assert_eq!(url, "https://api.anthropic.com/v1/messages?beta=true");
    }

    #[test]
    fn test_build_oauth_url_with_existing_params() {
        let url = build_oauth_url("https://api.anthropic.com/v1/messages?foo=bar");
        assert_eq!(url, "https://api.anthropic.com/v1/messages?foo=bar&beta=true");
    }

    #[test]
    fn test_prefix_tool_names_no_prefix() {
        let mut tools = json!([
            {"name": "search", "description": "Search"},
            {"name": "mcp_already_prefixed", "description": "Pre-prefixed"},
        ]);
        prefix_tool_names(&mut tools, MCP_TOOL_PREFIX);
        let arr = tools.as_array().unwrap();
        assert_eq!(arr[0]["name"], "mcp_search");
        assert_eq!(arr[1]["name"], "mcp_already_prefixed");
    }

    #[test]
    fn test_prefix_tool_use_in_messages() {
        let mut messages = json!([
            {
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Let me search"},
                    {"type": "tool_use", "id": "call_1", "name": "search", "input": {}},
                    {"type": "tool_use", "id": "call_2", "name": "mcp_already", "input": {}},
                ]
            }
        ]);
        prefix_tool_use_in_messages(&mut messages, MCP_TOOL_PREFIX);
        let content = &messages[0]["content"];
        assert_eq!(content[1]["name"], "mcp_search");
        assert_eq!(content[2]["name"], "mcp_already");
    }
}
