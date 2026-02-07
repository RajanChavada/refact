use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde_json::{json, Value};

use crate::call_validation::ChatUsage;
use crate::llm::adapter::{AdapterSettings, HttpParts, LlmWireAdapter, StreamParseError, extract_extra_fields, insert_extra_headers};
use crate::llm::canonical::{CanonicalToolChoice, LlmRequest, LlmStreamDelta};

pub struct RefactAdapter;

impl LlmWireAdapter for RefactAdapter {
    fn build_http(
        &self,
        req: &LlmRequest,
        settings: &AdapterSettings,
    ) -> Result<HttpParts, String> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if !settings.api_key.is_empty() {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", settings.api_key))
                    .map_err(|e| format!("invalid api_key for header: {e}"))?,
            );
        }

        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&format!("refact-lsp {}", env!("CARGO_PKG_VERSION")))
                .unwrap_or_else(|_| HeaderValue::from_static("refact-lsp")),
        );

        insert_extra_headers(&mut headers, &settings.extra_headers);

        let messages = convert_messages_to_refact(&req.messages);

        let mut body = json!({
            "model": settings.model_name,
            "messages": messages,
            "stream": req.stream,
        });

        if req.params.max_tokens > 0 {
            body["max_tokens"] = json!(req.params.max_tokens);
        }

        if let Some(temp) = req.params.temperature {
            body["temperature"] = json!(temp);
        }

        if let Some(freq_penalty) = req.params.frequency_penalty {
            body["frequency_penalty"] = json!(freq_penalty);
        }

        if !req.params.stop.is_empty() {
            body["stop"] = json!(req.params.stop);
        }

        if let Some(n) = req.params.n {
            body["n"] = json!(n);
        }

        if settings.supports_tools {
            if let Some(tools) = &req.tools {
                if !tools.is_empty() {
                    body["tools"] = json!(tools);
                    if let Some(choice) = &req.tool_choice {
                        body["tool_choice"] = tool_choice_to_refact(choice);
                    }
                    body["parallel_tool_calls"] = json!(req.parallel_tool_calls);
                }
            }
        }

        if settings.supports_reasoning {
            if let Some(effort) = req.reasoning.to_openai_effort() {
                body["reasoning_effort"] = json!(effort);
            }
        }

        if let Some(meta) = &req.meta {
            if let Ok(meta_value) = serde_json::to_value(meta) {
                body["meta"] = meta_value;
            }
        }

        if req.stream {
            body["stream_options"] = json!({"include_usage": true});
        }

        tracing::info!(
            model = %settings.model_name,
            endpoint = %settings.endpoint,
            stream = %req.stream,
            max_tokens = %req.params.max_tokens,
            temperature = ?req.params.temperature,
            frequency_penalty = ?req.params.frequency_penalty,
            n = ?req.params.n,
            stop_sequences = ?req.params.stop.len(),
            tools_count = ?req.tools.as_ref().map(|t| t.len()),
            tool_choice = ?req.tool_choice,
            reasoning = ?req.reasoning,
            has_meta = %req.meta.is_some(),
            messages_count = %req.messages.len(),
            "refact adapter request"
        );

        Ok(HttpParts {
            url: settings.endpoint.clone(),
            headers,
            body,
        })
    }

    fn parse_stream_chunk(&self, data: &str) -> Result<Vec<LlmStreamDelta>, StreamParseError> {
        let trimmed = data.trim();

        if trimmed.is_empty() {
            return Err(StreamParseError::Skip);
        }

        if trimmed == "[DONE]" {
            return Ok(vec![LlmStreamDelta::Done]);
        }

        let json: Value = serde_json::from_str(trimmed)
            .map_err(|e| StreamParseError::MalformedChunk(format!("json parse: {e}")))?;

        if let Some(error) = json.get("error") {
            return Err(StreamParseError::FatalError(
                error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown error")
                    .to_string(),
            ));
        }

        // FastAPI-style error (Refact backend uses FastAPI)
        if let Some(detail) = json.get("detail") {
            let msg = match detail {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            return Err(StreamParseError::FatalError(msg));
        }

        let mut deltas = Vec::new();

        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
            for choice in choices {
                if let Some(delta) = choice.get("delta") {
                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                        if !content.is_empty() {
                            deltas.push(LlmStreamDelta::AppendContent {
                                text: content.to_string(),
                            });
                        }
                    }

                    if let Some(reasoning) = delta.get("reasoning_content").and_then(|r| r.as_str()) {
                        if !reasoning.is_empty() {
                            deltas.push(LlmStreamDelta::AppendReasoning {
                                text: reasoning.to_string(),
                            });
                        }
                    }

                    if let Some(tool_calls) = delta.get("tool_calls") {
                        if let Some(arr) = tool_calls.as_array() {
                            if !arr.is_empty() {
                                deltas.push(LlmStreamDelta::SetToolCalls {
                                    tool_calls: arr.clone(),
                                });
                            }
                        }
                    }

                    // Citations support (Refact cloud via litellm)
                    if let Some(citations) = delta.get("citations") {
                        if let Some(arr) = citations.as_array() {
                            for citation in arr {
                                deltas.push(LlmStreamDelta::AddCitation {
                                    citation: citation.clone(),
                                });
                            }
                        }
                    }
                }

                if let Some(reason) = choice.get("finish_reason").and_then(|r| r.as_str()) {
                    deltas.push(LlmStreamDelta::SetFinishReason {
                        reason: reason.to_string(),
                    });
                }
            }
        }

        if let Some(usage) = json.get("usage") {
            if let Some(u) = parse_refact_usage(usage) {
                deltas.push(LlmStreamDelta::SetUsage { usage: u });
            }
        }

        let extra = extract_extra_fields(&json);
        if !extra.is_empty() {
            deltas.push(LlmStreamDelta::MergeExtra { extra });
        }

        Ok(deltas)
    }
}

fn convert_messages_to_refact(messages: &[crate::call_validation::ChatMessage]) -> Vec<Value> {
    messages
        .iter()
        .filter_map(|msg| {
            let role = match msg.role.as_str() {
                "user" | "assistant" | "system" | "tool" => msg.role.clone(),
                "diff" => "tool".to_string(),  // diff messages are tool results
                "plain_text" | "cd_instruction" | "context_file" => "user".to_string(),
                _ => return None,
            };

            // Filter out tool results for server-executed tools
            if (role == "tool" || msg.role == "diff") && msg.tool_call_id.starts_with("srvtoolu_") {
                return None;
            }

            let mut obj = json!({"role": role});

            match &msg.content {
                crate::call_validation::ChatContent::SimpleText(text) => {
                    obj["content"] = json!(text);
                }
                crate::call_validation::ChatContent::Multimodal(elements) => {
                    let content: Vec<Value> = elements
                        .iter()
                        .map(|el| {
                            if el.is_image() {
                                json!({
                                    "type": "image_url",
                                    "image_url": {
                                        "url": format!("data:{};base64,{}", el.m_type, el.m_content)
                                    }
                                })
                            } else {
                                json!({"type": "text", "text": el.m_content})
                            }
                        })
                        .collect();
                    obj["content"] = json!(content);
                }
                crate::call_validation::ChatContent::ContextFiles(files) => {
                    let text = files
                        .iter()
                        .map(|f| format!("{}:{}-{}\n```\n{}```", f.file_name, f.line1, f.line2, f.file_content))
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    obj["content"] = json!(text);
                }
            }

            if let Some(tool_calls) = &msg.tool_calls {
                let tc: Vec<Value> = tool_calls
                    .iter()
                    .filter(|tc| !tc.id.starts_with("srvtoolu_"))  // Filter server-executed tools
                    .map(|tc| {
                        json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.function.name,
                                "arguments": tc.function.arguments
                            }
                        })
                    })
                    .collect();
                if !tc.is_empty() {
                    obj["tool_calls"] = json!(tc);
                }
            }

            if !msg.tool_call_id.is_empty() {
                obj["tool_call_id"] = json!(msg.tool_call_id);
            }

            Some(obj)
        })
        .collect()
}

fn tool_choice_to_refact(choice: &CanonicalToolChoice) -> Value {
    match choice {
        CanonicalToolChoice::Auto => json!("auto"),
        CanonicalToolChoice::None => json!("none"),
        CanonicalToolChoice::Required => json!("required"),
        CanonicalToolChoice::Function { name } => json!({"type": "function", "function": {"name": name}}),
    }
}

fn parse_refact_usage(usage: &Value) -> Option<ChatUsage> {
    // Refact cloud via LiteLLM proxies various providers with different semantics.
    //
    // CRITICAL: LiteLLM streaming sends RAW provider usage without full normalization.
    //
    // LiteLLM's prompt_tokens semantics (from their docs):
    //   "prompt_tokens: These are all prompt tokens including cache-miss and cache-hit input tokens"
    //   BUT: cache_creation_input_tokens is NOT included in prompt_tokens!
    //
    // So for Anthropic via LiteLLM:
    //   prompt_tokens = input_tokens + cache_read_input_tokens (but NOT cache_creation)
    //   Total context = prompt_tokens + cache_creation_input_tokens
    //
    // Native Anthropic API (direct adapter):
    //   input_tokens: NON-cached input only
    //   Total context = input_tokens + cache_read + cache_creation
    //
    // OpenAI:
    //   prompt_tokens: TOTAL input (includes all cached)
    //   prompt_tokens_details.cached_tokens: informational (subset)
    //   Total context = prompt_tokens
    //
    // Detection strategy:
    // - If ONLY input_tokens (no prompt_tokens): Native Anthropic style
    // - If prompt_tokens exists: LiteLLM/OpenAI style
    //
    // NORMALIZATION: We always output prompt_tokens as NON-cached portion
    // and expose cache fields so UI can sum: prompt_tokens + cache_read + cache_creation

    let has_prompt_tokens = usage.get("prompt_tokens").is_some();
    let has_input_tokens = usage.get("input_tokens").is_some();

    let cache_creation = parse_token_value(usage.get("cache_creation_input_tokens"));
    let cache_read = parse_token_value(usage.get("cache_read_input_tokens"));
    let openai_cached = usage.get("prompt_tokens_details")
        .and_then(|d| parse_token_value(d.get("cached_tokens")));

    let effective_cache_read = cache_read.or(openai_cached);

    let completion_tokens = parse_token_value(usage.get("output_tokens"))
        .or_else(|| parse_token_value(usage.get("completion_tokens")))
        .unwrap_or(0);

    let provider_total = parse_token_value(usage.get("total_tokens"));

    let prompt_tokens = if let Some(total) = provider_total {
        let cache_sum = cache_creation.unwrap_or(0) + effective_cache_read.unwrap_or(0);
        total.saturating_sub(completion_tokens).saturating_sub(cache_sum)
    } else if has_prompt_tokens {
        let raw_prompt = parse_token_value(usage.get("prompt_tokens")).unwrap_or(0);
        raw_prompt.saturating_sub(effective_cache_read.unwrap_or(0))
    } else if has_input_tokens {
        parse_token_value(usage.get("input_tokens")).unwrap_or(0)
    } else {
        0
    };

    let total_tokens = provider_total.unwrap_or_else(|| {
        prompt_tokens
            + completion_tokens
            + cache_creation.unwrap_or(0)
            + effective_cache_read.unwrap_or(0)
    });

    let cache_creation_out = cache_creation.filter(|&v| v > 0);
    let cache_read_out = effective_cache_read.filter(|&v| v > 0);

    Some(ChatUsage {
        prompt_tokens,
        completion_tokens,
        total_tokens,
        cache_creation_tokens: cache_creation_out,
        cache_read_tokens: cache_read_out,
        metering_usd: None,
    })
}

fn parse_token_value(value: Option<&Value>) -> Option<usize> {
    value.and_then(|v| {
        v.as_u64()
            .map(|n| n as usize)
            .or_else(|| v.as_str().and_then(|s| s.parse::<usize>().ok()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_validation::{ChatMessage, ChatMeta};
    use reqwest::header::USER_AGENT;

    fn default_settings() -> AdapterSettings {
        AdapterSettings {
            api_key: "test-key".to_string(),
            endpoint: "https://app.refact.ai/v1/chat/completions".to_string(),
            extra_headers: Default::default(),
            model_name: "gpt-4".to_string(),
            supports_tools: true,
            supports_reasoning: false,
            supports_max_completion_tokens: false,
            support_metadata: true,
            eof_is_done: false,
        }
    }

    #[test]
    fn test_user_agent_format() {
        let adapter = RefactAdapter;
        let req = LlmRequest::new("gpt-4".to_string(), vec![
            ChatMessage::new("user".to_string(), "Hi".to_string()),
        ]);

        let http = adapter.build_http(&req, &default_settings()).unwrap();

        let ua = http.headers.get(USER_AGENT).unwrap().to_str().unwrap();
        assert!(ua.starts_with("refact-lsp "), "UA should use space separator");
    }

    #[test]
    fn test_meta_included_when_provided() {
        let adapter = RefactAdapter;
        let meta = ChatMeta {
            chat_id: "test-123".to_string(),
            chat_mode: "agent".to_string(),
            ..Default::default()
        };
        let req = LlmRequest::new("gpt-4".to_string(), vec![
            ChatMessage::new("user".to_string(), "Hi".to_string()),
        ]).with_meta(meta);

        let http = adapter.build_http(&req, &default_settings()).unwrap();

        assert!(http.body.get("meta").is_some());
        assert_eq!(http.body["meta"]["chat_id"], "test-123");
    }

    #[test]
    fn test_meta_not_included_when_absent() {
        let adapter = RefactAdapter;
        let req = LlmRequest::new("gpt-4".to_string(), vec![
            ChatMessage::new("user".to_string(), "Hi".to_string()),
        ]);

        let http = adapter.build_http(&req, &default_settings()).unwrap();

        assert!(http.body.get("meta").is_none());
    }

    #[test]
    fn test_no_backend_fields_in_request() {
        let adapter = RefactAdapter;
        let req = LlmRequest::new("gpt-4".to_string(), vec![
            ChatMessage::new("user".to_string(), "Hi".to_string()),
        ]);

        let http = adapter.build_http(&req, &default_settings()).unwrap();

        assert!(http.body.get("id").is_none());
        assert!(http.body.get("created").is_none());
        assert!(http.body.get("system_fingerprint").is_none());
    }

    #[test]
    fn test_stream_options_included() {
        let adapter = RefactAdapter;
        let req = LlmRequest::new("gpt-4".to_string(), vec![
            ChatMessage::new("user".to_string(), "Hi".to_string()),
        ]);

        let http = adapter.build_http(&req, &default_settings()).unwrap();

        assert_eq!(http.body["stream"], true);
        assert_eq!(http.body["stream_options"]["include_usage"], true);
    }

    #[test]
    fn test_parallel_tool_calls_included() {
        let adapter = RefactAdapter;
        let tools = vec![json!({"type": "function", "function": {"name": "test"}})];
        let req = LlmRequest::new("gpt-4".to_string(), vec![])
            .with_tools(tools, Some(CanonicalToolChoice::Auto));

        let http = adapter.build_http(&req, &default_settings()).unwrap();

        assert!(http.body.get("parallel_tool_calls").is_some());
    }

    #[test]
    fn test_parse_stream_with_metering() {
        let adapter = RefactAdapter;
        let chunk = r#"{"choices":[{"delta":{"content":"Hi"}}],"metering_balance":5000,"metering_prompt_tokens_n":10}"#;

        let deltas = adapter.parse_stream_chunk(chunk).unwrap();

        let has_content = deltas.iter().any(|d| matches!(d, LlmStreamDelta::AppendContent { text } if text == "Hi"));
        let has_extra = deltas.iter().any(|d| matches!(d, LlmStreamDelta::MergeExtra { extra } if extra.contains_key("metering_balance")));

        assert!(has_content);
        assert!(has_extra);
    }

    #[test]
    fn test_parse_fastapi_error_string() {
        let adapter = RefactAdapter;
        let chunk = r#"{"detail": "The version of your Refact plugin is no longer supported"}"#;

        let result = adapter.parse_stream_chunk(chunk);

        match result {
            Err(StreamParseError::FatalError(msg)) => {
                assert!(msg.contains("no longer supported"));
            }
            _ => panic!("expected FatalError"),
        }
    }

    #[test]
    fn test_parse_fastapi_error_object() {
        let adapter = RefactAdapter;
        let chunk = r#"{"detail": {"code": "version_error", "message": "Update required"}}"#;

        let result = adapter.parse_stream_chunk(chunk);

        match result {
            Err(StreamParseError::FatalError(msg)) => {
                assert!(msg.contains("version_error"));
            }
            _ => panic!("expected FatalError"),
        }
    }

    #[test]
    fn test_parse_openai_error() {
        let adapter = RefactAdapter;
        let chunk = r#"{"error": {"message": "Rate limit exceeded", "type": "rate_limit"}}"#;

        let result = adapter.parse_stream_chunk(chunk);

        match result {
            Err(StreamParseError::FatalError(msg)) => {
                assert_eq!(msg, "Rate limit exceeded");
            }
            _ => panic!("expected FatalError"),
        }
    }

    #[test]
    fn test_parse_done() {
        let adapter = RefactAdapter;
        let deltas = adapter.parse_stream_chunk("[DONE]").unwrap();

        assert_eq!(deltas.len(), 1);
        assert!(matches!(deltas[0], LlmStreamDelta::Done));
    }

    #[test]
    fn test_parse_tool_calls_delta() {
        let adapter = RefactAdapter;
        let chunk = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_123","function":{"name":"test","arguments":""}}]}}]}"#;

        let deltas = adapter.parse_stream_chunk(chunk).unwrap();

        let has_tool_calls = deltas.iter().any(|d| matches!(d, LlmStreamDelta::SetToolCalls { tool_calls } if !tool_calls.is_empty()));
        assert!(has_tool_calls);
    }

    #[test]
    fn test_n_parameter_included() {
        let adapter = RefactAdapter;
        let mut req = LlmRequest::new("gpt-4".to_string(), vec![
            ChatMessage::new("user".to_string(), "Hi".to_string()),
        ]);
        req.params.n = Some(2);

        let http = adapter.build_http(&req, &default_settings()).unwrap();

        assert_eq!(http.body["n"], 2);
    }

    #[test]
    fn test_diff_role_converted_to_tool() {
        let messages = vec![
            ChatMessage::new("user".to_string(), "Edit the file".to_string()),
            ChatMessage {
                role: "assistant".to_string(),
                content: crate::call_validation::ChatContent::SimpleText("".to_string()),
                tool_calls: Some(vec![crate::call_validation::ChatToolCall {
                    id: "call_edit".to_string(),
                    tool_type: "function".to_string(),
                    function: crate::call_validation::ChatToolFunction {
                        name: "file_edit".to_string(),
                        arguments: "{}".to_string(),
                    },
                    index: None,
                }]),
                ..Default::default()
            },
            ChatMessage {
                role: "diff".to_string(),
                content: crate::call_validation::ChatContent::SimpleText("@@ -1 +1 @@\n-old\n+new".to_string()),
                tool_call_id: "call_edit".to_string(),
                ..Default::default()
            },
        ];

        let converted = convert_messages_to_refact(&messages);

        assert_eq!(converted.len(), 3);
        assert_eq!(converted[2]["role"], "tool");
        assert_eq!(converted[2]["tool_call_id"], "call_edit");
    }

    #[test]
    fn test_stream_citations_in_delta() {
        let adapter = RefactAdapter;
        let chunk = r#"{"id":"123","choices":[{"index":0,"delta":{"citations":[{"url":"https://example.com","title":"Example","snippet":"Some text"}]}}]}"#;

        let deltas = adapter.parse_stream_chunk(chunk).unwrap();
        let citation_count = deltas.iter().filter(|d| matches!(d, LlmStreamDelta::AddCitation { .. })).count();
        assert_eq!(citation_count, 1);

        // Verify citation content
        if let Some(LlmStreamDelta::AddCitation { citation }) = deltas.iter().find(|d| matches!(d, LlmStreamDelta::AddCitation { .. })) {
            assert_eq!(citation.get("url").and_then(|v| v.as_str()), Some("https://example.com"));
            assert_eq!(citation.get("title").and_then(|v| v.as_str()), Some("Example"));
        }
    }

    #[test]
    fn test_parse_usage_litellm_anthropic_style_with_total() {
        let usage = serde_json::json!({
            "prompt_tokens": 1500,
            "completion_tokens": 200,
            "cache_read_input_tokens": 1000,
            "cache_creation_input_tokens": 300,
            "total_tokens": 2000
        });

        let parsed = parse_refact_usage(&usage).unwrap();

        assert_eq!(parsed.prompt_tokens, 500);
        assert_eq!(parsed.completion_tokens, 200);
        assert_eq!(parsed.cache_creation_tokens, Some(300));
        assert_eq!(parsed.cache_read_tokens, Some(1000));
        assert_eq!(parsed.total_tokens, 2000);
    }

    #[test]
    fn test_parse_usage_litellm_anthropic_style_no_total() {
        let usage = serde_json::json!({
            "prompt_tokens": 1500,
            "completion_tokens": 200,
            "cache_read_input_tokens": 1000,
            "cache_creation_input_tokens": 300
        });

        let parsed = parse_refact_usage(&usage).unwrap();

        assert_eq!(parsed.prompt_tokens, 500);
        assert_eq!(parsed.completion_tokens, 200);
        assert_eq!(parsed.cache_creation_tokens, Some(300));
        assert_eq!(parsed.cache_read_tokens, Some(1000));
        assert_eq!(parsed.total_tokens, 2000);
    }

    #[test]
    fn test_parse_usage_native_anthropic_style() {
        // Native Anthropic API: input_tokens is NON-cached, cache must be ADDED
        let usage = serde_json::json!({
            "input_tokens": 500,  // NON-cached only
            "output_tokens": 200,
            "cache_read_input_tokens": 1000,  // Must ADD to input_tokens
            "cache_creation_input_tokens": 300  // Must ADD to input_tokens
        });

        let parsed = parse_refact_usage(&usage).unwrap();

        // input_tokens used as prompt_tokens (non-cached)
        assert_eq!(parsed.prompt_tokens, 500);
        assert_eq!(parsed.completion_tokens, 200);
        // Cache fields SHOULD be exposed (they're additive)
        assert_eq!(parsed.cache_creation_tokens, Some(300));
        assert_eq!(parsed.cache_read_tokens, Some(1000));
        // Total = input + completion + cache_creation + cache_read
        assert_eq!(parsed.total_tokens, 2000);
    }

    #[test]
    fn test_parse_usage_openai_style_no_cache() {
        // Standard OpenAI without cache info
        let usage = serde_json::json!({
            "prompt_tokens": 1000,
            "completion_tokens": 200,
            "total_tokens": 1200
        });

        let parsed = parse_refact_usage(&usage).unwrap();

        assert_eq!(parsed.prompt_tokens, 1000);
        assert_eq!(parsed.completion_tokens, 200);
        assert_eq!(parsed.cache_creation_tokens, None);
        assert_eq!(parsed.cache_read_tokens, None);
        assert_eq!(parsed.total_tokens, 1200);
    }

    #[test]
    fn test_parse_usage_preserves_explicit_total() {
        // If provider sends explicit total_tokens, use it
        let usage = serde_json::json!({
            "prompt_tokens": 1000,
            "completion_tokens": 200,
            "total_tokens": 1500  // Might include reasoning tokens etc
        });

        let parsed = parse_refact_usage(&usage).unwrap();
        assert_eq!(parsed.total_tokens, 1500);
    }

    #[test]
    fn test_parse_usage_openai_with_cached_tokens_details() {
        let usage = serde_json::json!({
            "prompt_tokens": 1500,
            "completion_tokens": 200,
            "prompt_tokens_details": {
                "cached_tokens": 1000
            },
            "total_tokens": 1700
        });

        let parsed = parse_refact_usage(&usage).unwrap();

        assert_eq!(parsed.prompt_tokens, 500);
        assert_eq!(parsed.completion_tokens, 200);
        assert_eq!(parsed.cache_read_tokens, Some(1000));
        assert_eq!(parsed.cache_creation_tokens, None);
        assert_eq!(parsed.total_tokens, 1700);
    }

    #[test]
    fn test_parse_usage_string_numbers() {
        let usage = serde_json::json!({
            "prompt_tokens": "1000",
            "completion_tokens": "200",
            "total_tokens": "1200"
        });

        let parsed = parse_refact_usage(&usage).unwrap();

        assert_eq!(parsed.prompt_tokens, 1000);
        assert_eq!(parsed.completion_tokens, 200);
        assert_eq!(parsed.total_tokens, 1200);
    }
}
