use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::caps::model_caps::ModelCapabilities;
use crate::llm::adapter::WireFormat;
use crate::providers::openai_codex_oauth::OAuthTokens;
use crate::providers::traits::{
    AvailableModel, CustomModelConfig, ModelPricing, ModelSource, ProviderRuntime, ProviderTrait,
    merge_custom_models, parse_enabled_models, parse_custom_models, set_model_enabled_impl,
};

const CODEX_ORIGINATOR: &str = "refact-lsp";
const CHATGPT_CODEX_MODELS_URL: &str =
    "https://chatgpt.com/backend-api/codex/models?client_version=999.999.999";
const OPENAI_MODELS_URL: &str = "https://api.openai.com/v1/models";

fn new_codex_session_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn is_codex_model(id: &str) -> bool {
    id.to_lowercase().contains("codex")
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AuthSource {
    InAppOAuth,
    CodexCli,
    None,
}

#[derive(Debug, Clone)]
enum CodexAuth {
    PlatformApiKey {
        api_key: String,
    },
    ChatGptBackendOAuth {
        access_token: String,
        chatgpt_account_id: String,
    },
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICodexProvider {
    #[serde(default)]
    pub enabled_models: Vec<String>,
    #[serde(default)]
    pub custom_models: HashMap<String, CustomModelConfig>,
    #[serde(default)]
    pub oauth_tokens: OAuthTokens,
    #[serde(default = "new_codex_session_id")]
    pub session_id: String,
}

impl Default for OpenAICodexProvider {
    fn default() -> Self {
        Self {
            enabled_models: Vec::new(),
            custom_models: HashMap::new(),
            oauth_tokens: OAuthTokens::default(),
            session_id: new_codex_session_id(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAICodexUsageWindow {
    pub used_percent: f64,
    pub reset_at: Option<String>,
    pub limit_window_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAICodexRateLimit {
    pub limit_reached: bool,
    pub primary_window: Option<OpenAICodexUsageWindow>,
    pub secondary_window: Option<OpenAICodexUsageWindow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAICodexCredits {
    pub balance: f64,
    pub unlimited: bool,
    pub has_credits: bool,
    pub granted: Option<f64>,
    pub used: Option<f64>,
    pub reset_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAICodexUsage {
    pub plan_type: Option<String>,
    pub rate_limit: Option<OpenAICodexRateLimit>,
    pub code_review_rate_limit: Option<OpenAICodexRateLimit>,
    pub credits: Option<OpenAICodexCredits>,
}

impl OpenAICodexProvider {
    fn needs_refresh_on_start(expires_at: i64) -> bool {
        const REFRESH_BEFORE_EXPIRY_MS: i64 = 5 * 60 * 1000;
        if expires_at == 0 {
            return true;
        }
        let now_ms = chrono::Utc::now().timestamp_millis();
        now_ms >= expires_at - REFRESH_BEFORE_EXPIRY_MS
    }

    async fn save_oauth_tokens_config(&self, config_dir: &std::path::Path) -> Result<(), String> {
        let providers_dir = config_dir.join("providers.d");
        let config_path = providers_dir.join("openai_codex.yaml");

        tokio::fs::create_dir_all(&providers_dir)
            .await
            .map_err(|e| format!("Failed to create providers.d: {}", e))?;

        let mut yaml_map: serde_yaml::Mapping = if config_path.exists() {
            let content = tokio::fs::read_to_string(&config_path)
                .await
                .map_err(|e| format!("Failed to read config: {}", e))?;
            let value: serde_yaml::Value = serde_yaml::from_str(&content)
                .map_err(|e| format!("Failed to parse YAML: {}", e))?;
            value.as_mapping().cloned().ok_or_else(|| {
                "Config file root is not a YAML mapping. Cannot safely patch.".to_string()
            })?
        } else {
            serde_yaml::Mapping::new()
        };

        let mut tokens_map = yaml_map
            .get(&serde_yaml::Value::String("oauth_tokens".to_string()))
            .and_then(|v| v.as_mapping())
            .cloned()
            .unwrap_or_default();

        tokens_map.insert(
            serde_yaml::Value::String("access_token".to_string()),
            serde_yaml::Value::String(self.oauth_tokens.access_token.clone()),
        );
        tokens_map.insert(
            serde_yaml::Value::String("refresh_token".to_string()),
            serde_yaml::Value::String(self.oauth_tokens.refresh_token.clone()),
        );
        tokens_map.insert(
            serde_yaml::Value::String("expires_at".to_string()),
            serde_yaml::Value::Number(serde_yaml::Number::from(self.oauth_tokens.expires_at)),
        );
        tokens_map.insert(
            serde_yaml::Value::String("openai_api_key".to_string()),
            serde_yaml::Value::String(self.oauth_tokens.openai_api_key.clone()),
        );
        tokens_map.insert(
            serde_yaml::Value::String("chatgpt_account_id".to_string()),
            serde_yaml::Value::String(self.oauth_tokens.chatgpt_account_id.clone()),
        );
        tokens_map.insert(
            serde_yaml::Value::String("api_key_exchange_error".to_string()),
            serde_yaml::Value::String(self.oauth_tokens.api_key_exchange_error.clone()),
        );

        yaml_map.insert(
            serde_yaml::Value::String("oauth_tokens".to_string()),
            serde_yaml::Value::Mapping(tokens_map),
        );
        yaml_map.insert(
            serde_yaml::Value::String("session_id".to_string()),
            serde_yaml::Value::String(self.session_id.clone()),
        );

        let content = serde_yaml::to_string(&yaml_map)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique_id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_path = config_path.with_extension(format!(
            "yaml.tmp.oauth.{}.{}",
            std::process::id(),
            unique_id
        ));

        tokio::fs::write(&temp_path, &content)
            .await
            .map_err(|e| format!("Failed to write temp config: {}", e))?;
        tokio::fs::rename(&temp_path, &config_path)
            .await
            .map_err(|e| format!("Failed to rename config: {}", e))?;

        Ok(())
    }

    fn resolve_auth(&self) -> (AuthSource, CodexAuth) {
        if !self.oauth_tokens.openai_api_key.is_empty() {
            return (
                AuthSource::InAppOAuth,
                CodexAuth::PlatformApiKey {
                    api_key: self.oauth_tokens.openai_api_key.clone(),
                },
            );
        }

        if self.oauth_tokens.has_valid_access_token()
            && !self.oauth_tokens.chatgpt_account_id.is_empty()
        {
            return (
                AuthSource::InAppOAuth,
                CodexAuth::ChatGptBackendOAuth {
                    access_token: self.oauth_tokens.access_token.clone(),
                    chatgpt_account_id: self.oauth_tokens.chatgpt_account_id.clone(),
                },
            );
        }

        if let Ok(cli_tokens) = crate::providers::openai_codex_oauth::read_codex_cli_credentials() {
            if !cli_tokens.openai_api_key.is_empty() {
                return (
                    AuthSource::CodexCli,
                    CodexAuth::PlatformApiKey {
                        api_key: cli_tokens.openai_api_key,
                    },
                );
            }
        }

        if self.oauth_tokens.has_valid_access_token() {
            return (
                AuthSource::InAppOAuth,
                CodexAuth::ChatGptBackendOAuth {
                    access_token: self.oauth_tokens.access_token.clone(),
                    chatgpt_account_id: String::new(),
                },
            );
        }

        (AuthSource::None, CodexAuth::None)
    }

    fn resolve_wham_token(&self) -> Result<String, String> {
        if self.oauth_tokens.has_valid_access_token() {
            return Ok(self.oauth_tokens.access_token.clone());
        }
        if let Ok(cli_tokens) = crate::providers::openai_codex_oauth::read_codex_cli_credentials() {
            if !cli_tokens.access_token.is_empty() {
                return Ok(cli_tokens.access_token);
            }
        }
        Err("No ChatGPT OAuth access token available for usage API".to_string())
    }

    pub async fn fetch_usage(
        &self,
        http_client: &reqwest::Client,
    ) -> Result<OpenAICodexUsage, String> {
        let token = self.resolve_wham_token()?;

        let mut req = http_client
            .get("https://chatgpt.com/backend-api/wham/usage")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json");
        for (key, value) in self.chatgpt_backend_headers(&self.oauth_tokens.chatgpt_account_id) {
            req = req.header(key, value);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("OpenAI Codex usage request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let truncated: String = body.chars().take(512).collect();
            return Err(format!(
                "OpenAI Codex usage API returned {}. Check OpenAI Codex login/setup: {}",
                status, truncated
            ));
        }

        let root: Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse OpenAI Codex usage response: {}", e))?;

        Ok(Self::parse_usage_payload(&root))
    }

    fn parse_usage_payload(root: &Value) -> OpenAICodexUsage {
        let data = root.get("data").unwrap_or(root);
        let plan_type = Self::string_field(data, &["plan_type", "planType", "codex_plan_type"])
            .or_else(|| {
                data.get("plan")
                    .and_then(|plan| Self::string_field(plan, &["type", "name"]))
            });
        let rate_limit =
            Self::field(data, &["rate_limit", "rateLimit"]).map(Self::parse_rate_limit);
        let code_review_rate_limit = Self::field(
            data,
            &[
                "code_review_rate_limit",
                "codeReviewRateLimit",
                "code_review",
            ],
        )
        .map(Self::parse_rate_limit);
        let credits = Self::field(data, &["credits", "credit_balance"]).map(Self::parse_credits);
        OpenAICodexUsage {
            plan_type,
            rate_limit,
            code_review_rate_limit,
            credits,
        }
    }

    fn parse_rate_limit(rl: &Value) -> OpenAICodexRateLimit {
        let primary_window =
            Self::field(rl, &["primary_window", "primary"]).and_then(Self::parse_usage_window);
        let secondary_window =
            Self::field(rl, &["secondary_window", "secondary"]).and_then(Self::parse_usage_window);
        let limit_reached = Self::field(rl, &["limit_reached", "limitReached"])
            .and_then(Value::as_bool)
            .unwrap_or_else(|| {
                primary_window
                    .as_ref()
                    .map(|window| window.used_percent >= 100.0)
                    .unwrap_or(false)
                    || secondary_window
                        .as_ref()
                        .map(|window| window.used_percent >= 100.0)
                        .unwrap_or(false)
            });
        OpenAICodexRateLimit {
            limit_reached,
            primary_window,
            secondary_window,
        }
    }

    fn parse_usage_window(obj: &Value) -> Option<OpenAICodexUsageWindow> {
        let used_percent =
            Self::field(obj, &["used_percent", "usedPercent"]).and_then(Self::as_f64_loose)?;
        let reset_at = Self::field(obj, &["reset_at", "resets_at", "resetsAt"])
            .and_then(Self::timestamp_or_string);
        let limit_window_seconds = Self::field(
            obj,
            &[
                "limit_window_seconds",
                "limitWindowSeconds",
                "window_seconds",
            ],
        )
        .and_then(Self::as_u64_loose);
        Some(OpenAICodexUsageWindow {
            used_percent,
            reset_at,
            limit_window_seconds,
        })
    }

    fn parse_credits(c: &Value) -> OpenAICodexCredits {
        let balance = Self::field(c, &["balance", "remaining", "remaining_credits"])
            .and_then(Self::as_f64_loose)
            .unwrap_or(0.0);
        OpenAICodexCredits {
            balance,
            unlimited: Self::field(c, &["unlimited", "is_unlimited"])
                .and_then(Value::as_bool)
                .unwrap_or(false),
            has_credits: Self::field(c, &["has_credits", "hasCredits"])
                .and_then(Value::as_bool)
                .unwrap_or(balance > 0.0),
            granted: Self::field(c, &["granted", "total_granted", "total"])
                .and_then(Self::as_f64_loose),
            used: Self::field(c, &["used", "total_used"]).and_then(Self::as_f64_loose),
            reset_at: Self::field(c, &["reset_at", "expires_at", "expiresAt"])
                .and_then(Self::timestamp_or_string),
        }
    }

    fn field<'a>(obj: &'a Value, keys: &[&str]) -> Option<&'a Value> {
        keys.iter().find_map(|key| obj.get(*key))
    }

    fn string_field(obj: &Value, keys: &[&str]) -> Option<String> {
        Self::field(obj, keys).and_then(|value| value.as_str().map(ToString::to_string))
    }

    fn as_f64_loose(v: &Value) -> Option<f64> {
        v.as_f64()
            .or_else(|| v.as_i64().map(|i| i as f64))
            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
    }

    fn as_u64_loose(v: &Value) -> Option<u64> {
        v.as_u64()
            .or_else(|| v.as_i64().and_then(|i| (i >= 0).then_some(i as u64)))
            .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
    }

    fn timestamp_or_string(v: &Value) -> Option<String> {
        if let Some(s) = v.as_str() {
            return Some(s.to_string());
        }
        let ts = v.as_i64()?;
        if ts < 0 {
            return None;
        }
        let secs = if ts > 1_000_000_000_000 {
            ts / 1000
        } else {
            ts
        };
        use std::time::{Duration, UNIX_EPOCH};
        let dt: chrono::DateTime<chrono::Utc> =
            (UNIX_EPOCH + Duration::from_secs(secs as u64)).into();
        Some(dt.to_rfc3339())
    }

    fn diagnose_auth_status(&self) -> String {
        if !self.oauth_tokens.openai_api_key.is_empty() {
            return "OK (OAuth login — Platform API key)".to_string();
        }

        if self.oauth_tokens.has_valid_access_token() {
            if !self.oauth_tokens.chatgpt_account_id.is_empty() {
                if self.oauth_tokens.api_key_exchange_error.is_empty() {
                    return "Connected (ChatGPT backend)".to_string();
                }
                return "Connected (ChatGPT backend). Platform API key not available for this account.".to_string();
            }
            return "OAuth login incomplete: missing chatgpt_account_id".to_string();
        }

        if !self.oauth_tokens.is_empty() && self.oauth_tokens.has_refresh_token() {
            return "OAuth token expired — needs refresh".to_string();
        }
        if crate::providers::openai_codex_oauth::codex_cli_credentials_exist() {
            return "OK (Codex CLI session)".to_string();
        }
        "No credentials found".to_string()
    }

    fn chatgpt_backend_headers(&self, chatgpt_account_id: &str) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        if !chatgpt_account_id.is_empty() {
            headers.insert(
                "chatgpt-account-id".to_string(),
                chatgpt_account_id.to_string(),
            );
        }
        headers.insert(
            "OpenAI-Beta".to_string(),
            "responses=experimental".to_string(),
        );
        headers.insert("originator".to_string(), CODEX_ORIGINATOR.to_string());
        headers.insert("session_id".to_string(), self.session_id.clone());
        headers.insert("accept".to_string(), "text/event-stream".to_string());
        headers
    }

    async fn fetch_models_from_chatgpt_api(
        &self,
        http_client: &reqwest::Client,
        model_caps: &HashMap<String, ModelCapabilities>,
        access_token: &str,
        chatgpt_account_id: &str,
    ) -> Vec<AvailableModel> {
        let mut req = http_client.get(CHATGPT_CODEX_MODELS_URL).header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {access_token}"),
        );
        for (key, value) in self.chatgpt_backend_headers(chatgpt_account_id) {
            req = req.header(key, value);
        }

        let response = match req.send().await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::warn!("OpenAI Codex: failed to reach chatgpt backend /codex/models (network error): {}, using models.dev catalog fallback", e);
                return self.fetch_models_from_catalog(model_caps);
            }
        };

        let status = response.status();

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            tracing::warn!("OpenAI Codex: /codex/models returned {}. Refresh will be attempted on runtime 401/403; using models.dev catalog fallback for model list", status);
            return self.fetch_models_from_catalog(model_caps);
        }

        if !status.is_success() {
            tracing::warn!(
                "OpenAI Codex: /codex/models returned {} (transient), using models.dev catalog fallback",
                status
            );
            return self.fetch_models_from_catalog(model_caps);
        }

        let json: Value = match response.json().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("OpenAI Codex: failed to parse /codex/models response: {}, using models.dev catalog fallback", e);
                return self.fetch_models_from_catalog(model_caps);
            }
        };

        let Some(models_array) = Self::models_array_from_live_response(&json) else {
            tracing::warn!("OpenAI Codex: /codex/models response missing a model array, using models.dev catalog fallback");
            return self.fetch_models_from_catalog(model_caps);
        };

        let enabled_set: HashSet<&str> = self.enabled_models.iter().map(|s| s.as_str()).collect();
        let mut models_map = self.catalog_model_map(model_caps, &enabled_set);

        for model in models_array {
            let Some(slug) = Self::live_model_id(model) else {
                continue;
            };
            let supported_in_api = model
                .get("supported_in_api")
                .or_else(|| model.get("supportedInApi"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            if !supported_in_api {
                continue;
            }
            let enabled = enabled_set.contains(slug);
            let pricing = self.custom_model_pricing(slug);
            let display_name = Self::live_model_display_name(model);
            let mut available = if let Some(caps) =
                crate::caps::model_caps::resolve_model_caps(model_caps, &format!("openai/{slug}"))
            {
                AvailableModel::from_caps(slug, &caps.caps, enabled, pricing)
            } else if let Some(caps) = crate::caps::model_caps::resolve_model_caps(model_caps, slug)
            {
                AvailableModel::from_caps(slug, &caps.caps, enabled, pricing)
            } else {
                let mut m = self.default_codex_model(slug.to_string(), enabled, pricing);
                if let Some(ctx) = Self::live_model_context_window(model) {
                    m.n_ctx = ctx;
                }
                if let Some(supports_parallel) = model
                    .get("supports_parallel_tool_calls")
                    .or_else(|| model.get("supportsParallelToolCalls"))
                    .and_then(|v| v.as_bool())
                {
                    m.supports_parallel_tools = supports_parallel;
                }
                if let Some(input_modalities) =
                    model.get("input_modalities").and_then(|v| v.as_array())
                {
                    m.supports_multimodality = input_modalities
                        .iter()
                        .any(|modality| modality.as_str() == Some("image"));
                }
                if let Some(levels) = Self::live_model_reasoning_levels(model) {
                    m.reasoning_effort_options = Some(levels);
                }
                m
            };
            available.display_name = display_name.or(available.display_name);
            models_map.insert(slug.to_string(), available);
        }

        tracing::info!(
            "OpenAI Codex: {} models available (chatgpt backend + models.dev catalog)",
            models_map.len()
        );

        self.finish_models(models_map, &enabled_set)
    }

    async fn fetch_models_from_api(
        &self,
        http_client: &reqwest::Client,
        model_caps: &HashMap<String, ModelCapabilities>,
        api_key: &str,
    ) -> Vec<AvailableModel> {
        let response = match http_client
            .get(OPENAI_MODELS_URL)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {api_key}"))
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                tracing::warn!("OpenAI Codex: failed to reach /v1/models (network error): {}, using models.dev catalog fallback", e);
                return self.fetch_models_from_catalog(model_caps);
            }
        };

        let status = response.status();

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            tracing::warn!("OpenAI Codex: /v1/models returned {}. Check OpenAI Codex provider setup or API key exchange; returning custom models only", status);
            return self.get_custom_models_only();
        }

        if !status.is_success() {
            tracing::warn!(
                "OpenAI Codex: /v1/models returned {} (transient), using models.dev catalog fallback",
                status
            );
            return self.fetch_models_from_catalog(model_caps);
        }

        let json: Value = match response.json().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    "OpenAI Codex: failed to parse /v1/models response: {}, using models.dev catalog fallback",
                    e
                );
                return self.fetch_models_from_catalog(model_caps);
            }
        };

        let Some(data_array) = Self::models_array_from_live_response(&json) else {
            tracing::warn!("OpenAI Codex: /v1/models response missing a model array, using models.dev catalog fallback");
            return self.fetch_models_from_catalog(model_caps);
        };

        let enabled_set: HashSet<&str> = self.enabled_models.iter().map(|s| s.as_str()).collect();
        let mut models_map = self.catalog_model_map(model_caps, &enabled_set);

        for model in data_array {
            let Some(id) = Self::live_model_id(model) else {
                continue;
            };
            if !is_codex_model(id) {
                continue;
            }
            let enabled = enabled_set.contains(id);
            let pricing = self.custom_model_pricing(id);
            let mut available = if let Some(caps) =
                crate::caps::model_caps::resolve_model_caps(model_caps, &format!("openai/{id}"))
            {
                AvailableModel::from_caps(id, &caps.caps, enabled, pricing)
            } else if let Some(caps) = crate::caps::model_caps::resolve_model_caps(model_caps, id) {
                AvailableModel::from_caps(id, &caps.caps, enabled, pricing)
            } else {
                self.default_codex_model(id.to_string(), enabled, pricing)
            };
            available.display_name =
                Self::live_model_display_name(model).or(available.display_name);
            models_map.insert(id.to_string(), available);
        }

        tracing::info!(
            "OpenAI Codex: {} models available (/v1/models + models.dev catalog)",
            models_map.len()
        );

        self.finish_models(models_map, &enabled_set)
    }

    fn fetch_models_from_catalog(
        &self,
        model_caps: &HashMap<String, ModelCapabilities>,
    ) -> Vec<AvailableModel> {
        let enabled_set: HashSet<&str> = self.enabled_models.iter().map(|s| s.as_str()).collect();
        let models_map = self.catalog_model_map(model_caps, &enabled_set);
        tracing::info!(
            "OpenAI Codex: {} models available (models.dev catalog fallback)",
            models_map.len()
        );
        self.finish_models(models_map, &enabled_set)
    }

    fn catalog_model_map(
        &self,
        model_caps: &HashMap<String, ModelCapabilities>,
        enabled_set: &HashSet<&str>,
    ) -> HashMap<String, AvailableModel> {
        let has_openai_qualified_caps = model_caps.keys().any(|key| key.starts_with("openai/"));
        let mut models_map: HashMap<String, AvailableModel> = HashMap::new();
        for (capability_key, caps) in model_caps {
            let model_id = if let Some(model_id) = capability_key.strip_prefix("openai/") {
                model_id
            } else if has_openai_qualified_caps || capability_key.contains('/') {
                continue;
            } else {
                capability_key.as_str()
            };
            if !is_codex_model(model_id) {
                continue;
            }
            let enabled =
                enabled_set.contains(model_id) || enabled_set.contains(capability_key.as_str());
            let pricing = self
                .custom_model_pricing(model_id)
                .or_else(|| self.custom_model_pricing(capability_key));
            models_map.insert(
                model_id.to_string(),
                AvailableModel::from_caps(model_id, caps, enabled, pricing),
            );
        }
        models_map
    }

    fn finish_models(
        &self,
        mut models_map: HashMap<String, AvailableModel>,
        enabled_set: &HashSet<&str>,
    ) -> Vec<AvailableModel> {
        let mut models: Vec<AvailableModel> = models_map.drain().map(|(_, model)| model).collect();
        merge_custom_models(&mut models, &self.custom_models, enabled_set);
        models.sort_by(|a, b| a.id.cmp(&b.id));
        models
    }

    fn models_array_from_live_response(json: &Value) -> Option<&Vec<Value>> {
        json.get("models")
            .or_else(|| json.get("data"))
            .and_then(Value::as_array)
    }

    fn live_model_id(model: &Value) -> Option<&str> {
        model
            .get("slug")
            .or_else(|| model.get("id"))
            .or_else(|| model.get("model"))
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
    }

    fn live_model_display_name(model: &Value) -> Option<String> {
        model
            .get("display_name")
            .or_else(|| model.get("displayName"))
            .or_else(|| model.get("name"))
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(ToString::to_string)
    }

    fn live_model_context_window(model: &Value) -> Option<usize> {
        model
            .get("max_context_window")
            .or_else(|| model.get("context_window"))
            .or_else(|| model.get("contextWindow"))
            .and_then(Value::as_u64)
            .map(|v| v as usize)
    }

    fn live_model_reasoning_levels(model: &Value) -> Option<Vec<String>> {
        let levels = model
            .get("supported_reasoning_levels")
            .or_else(|| model.get("supportedReasoningLevels"))
            .and_then(Value::as_array)?
            .iter()
            .filter_map(|r| {
                r.get("effort")
                    .or_else(|| r.get("id"))
                    .or_else(|| r.get("name"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
            .collect::<Vec<_>>();
        (!levels.is_empty()).then_some(levels)
    }

    pub(crate) fn should_force_refresh_for_status(
        status: reqwest::StatusCode,
        refresh_token: &str,
        already_attempted: bool,
    ) -> bool {
        !already_attempted
            && !refresh_token.is_empty()
            && matches!(
                status,
                reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN
            )
    }

    fn merge_refreshed_tokens(&self, mut refreshed: OAuthTokens) -> OAuthTokens {
        if refreshed.openai_api_key.is_empty() {
            refreshed.openai_api_key = self.oauth_tokens.openai_api_key.clone();
        }
        if refreshed.chatgpt_account_id.is_empty() {
            refreshed.chatgpt_account_id = self.oauth_tokens.chatgpt_account_id.clone();
        }
        if refreshed.api_key_exchange_error.is_empty() {
            refreshed.api_key_exchange_error = self.oauth_tokens.api_key_exchange_error.clone();
        }
        refreshed
    }

    pub(crate) fn clear_tokens_after_permanent_refresh_error(&mut self) {
        self.oauth_tokens.access_token.clear();
        self.oauth_tokens.refresh_token.clear();
        self.oauth_tokens.expires_at = 0;
    }

    pub(crate) async fn force_refresh_after_auth_rejection(
        &mut self,
        http_client: &reqwest::Client,
        config_dir: &std::path::Path,
    ) -> Result<Option<String>, String> {
        if self.oauth_tokens.refresh_token.is_empty() {
            return Ok(None);
        }

        let refreshed = match crate::providers::openai_codex_oauth::refresh_access_token(
            http_client,
            &self.oauth_tokens.refresh_token,
        )
        .await
        {
            Ok(refreshed) => refreshed,
            Err(e) if crate::providers::oauth_refresh::is_permanent_refresh_error(&e) => {
                crate::providers::oauth_refresh::mark_invalid_refresh_token(
                    "openai_codex",
                    &self.oauth_tokens.refresh_token,
                );
                self.clear_tokens_after_permanent_refresh_error();
                self.save_oauth_tokens_config(config_dir).await?;
                return Err(format!(
                    "OpenAI Codex OAuth refresh token is invalid. Please log in again in OpenAI Codex provider settings: {}",
                    e
                ));
            }
            Err(e) => {
                return Err(format!(
                    "OpenAI Codex OAuth refresh failed after backend rejected the access token: {}",
                    e
                ));
            }
        };

        let refreshed = self.merge_refreshed_tokens(refreshed);
        let access_token = refreshed.access_token.clone();
        self.oauth_tokens = refreshed;
        self.save_oauth_tokens_config(config_dir).await?;
        Ok((!access_token.is_empty()).then_some(access_token))
    }

    fn default_codex_model(
        &self,
        id: String,
        enabled: bool,
        pricing: Option<ModelPricing>,
    ) -> AvailableModel {
        AvailableModel {
            id,
            display_name: None,
            n_ctx: 200_000,
            supports_tools: true,
            supports_parallel_tools: true,
            supports_strict_tools: false,
            supports_multimodality: true,
            reasoning_effort_options: Some(vec![
                "low".to_string(),
                "medium".to_string(),
                "high".to_string(),
            ]),
            supports_thinking_budget: false,
            supports_adaptive_thinking_budget: false,
            tokenizer: None,
            enabled,
            is_custom: false,
            pricing,
            available_providers: Vec::new(),
            selected_provider: None,
            max_output_tokens: None,
            provider_variants: Vec::new(),
            wire_format_override: None,
            endpoint_override: None,
            base_model: None,
        }
    }
}

#[async_trait]
impl ProviderTrait for OpenAICodexProvider {
    fn name(&self) -> &'static str {
        "openai_codex"
    }

    fn display_name(&self) -> &'static str {
        "OpenAI Codex"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn ProviderTrait> {
        Box::new(self.clone())
    }

    fn default_wire_format(&self) -> WireFormat {
        WireFormat::OpenaiResponses
    }

    fn model_filter_regex(&self) -> Option<&'static str> {
        Some(r"(?i)^gpt.*codex")
    }

    fn provider_schema(&self) -> &'static str {
        r#"
fields: {}
oauth:
  supported: true
  methods:
    - id: chatgpt
      label: "ChatGPT Plus/Pro"
      description: "Login with your ChatGPT Plus or Pro subscription"
description: |
  Use your ChatGPT Plus/Pro subscription to access OpenAI Codex models (GPT-5-Codex family).

  **Setup:** Click **Login with OpenAI** below, or install Codex CLI and run `codex login`.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
"#
    }

    fn provider_settings_apply(&mut self, yaml: serde_yaml::Value) -> Result<(), String> {
        if let Some(oauth_tokens) = yaml.get("oauth_tokens") {
            self.oauth_tokens = serde_yaml::from_value(oauth_tokens.clone()).unwrap_or_default();
        }
        if let Some(session_id) = yaml
            .get("session_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            self.session_id = session_id.to_string();
        }
        parse_enabled_models(&yaml, &mut self.enabled_models);
        parse_custom_models(&yaml, &mut self.custom_models);
        Ok(())
    }

    fn provider_settings_as_json(&self) -> serde_json::Value {
        let auth_status = self.diagnose_auth_status();
        let oauth_connected =
            self.oauth_tokens.has_valid_access_token() || self.oauth_tokens.has_refresh_token();
        let api_key_ready = !self.oauth_tokens.openai_api_key.is_empty();

        json!({
            "auth_status": auth_status,
            "oauth_connected": oauth_connected,
            "api_key_ready": api_key_ready,
            "api_key_exchange_error": self.oauth_tokens.api_key_exchange_error,
            "enabled_models": self.enabled_models,
            "custom_models": self.custom_models
        })
    }

    fn build_runtime(&self) -> Result<ProviderRuntime, String> {
        let (_, auth) = self.resolve_auth();
        let mut extra_headers = HashMap::new();

        let (chat_endpoint, api_key) = match auth {
            CodexAuth::PlatformApiKey { api_key } => {
                ("https://api.openai.com/v1/responses".to_string(), api_key)
            }
            CodexAuth::ChatGptBackendOAuth {
                access_token,
                chatgpt_account_id,
                ..
            } => {
                extra_headers = self.chatgpt_backend_headers(&chatgpt_account_id);
                (
                    "https://chatgpt.com/backend-api/codex/responses".to_string(),
                    access_token,
                )
            }
            CodexAuth::None => (String::new(), String::new()),
        };

        let has_auth = !api_key.is_empty() && !chat_endpoint.is_empty();

        Ok(ProviderRuntime {
            name: self.name().to_string(),
            display_name: self.display_name().to_string(),
            enabled: has_auth && !self.enabled_models.is_empty(),
            readonly: false,
            wire_format: self.default_wire_format(),
            chat_endpoint,
            completion_endpoint: String::new(),
            embedding_endpoint: String::new(),
            api_key,
            auth_token: String::new(),
            tokenizer_api_key: String::new(),
            extra_headers,
            supports_cache_control: true,
            chat_models: Vec::new(),
            completion_models: Vec::new(),
            embedding_model: None,
        })
    }

    fn has_credentials(&self) -> bool {
        if !self.oauth_tokens.openai_api_key.is_empty() {
            return true;
        }
        if self.oauth_tokens.has_valid_access_token() {
            return true;
        }
        if self.oauth_tokens.has_refresh_token() {
            return true;
        }
        crate::providers::openai_codex_oauth::codex_cli_credentials_exist()
    }

    fn model_source(&self) -> ModelSource {
        let (_, auth) = self.resolve_auth();
        match auth {
            CodexAuth::PlatformApiKey { ref api_key } if !api_key.is_empty() => ModelSource::Api,
            _ => ModelSource::ModelCaps,
        }
    }

    fn enabled_models(&self) -> &[String] {
        &self.enabled_models
    }

    fn custom_models(&self) -> &HashMap<String, CustomModelConfig> {
        &self.custom_models
    }

    async fn fetch_available_models(
        &self,
        http_client: &reqwest::Client,
        model_caps: &HashMap<String, ModelCapabilities>,
    ) -> Vec<AvailableModel> {
        let (_, auth) = self.resolve_auth();
        match auth {
            CodexAuth::None => {
                tracing::warn!("OpenAI Codex: no auth");
                return self.get_custom_models_only();
            }
            CodexAuth::PlatformApiKey { ref api_key } if !api_key.is_empty() => {
                return self
                    .fetch_models_from_api(http_client, model_caps, api_key)
                    .await;
            }
            CodexAuth::ChatGptBackendOAuth {
                ref access_token,
                ref chatgpt_account_id,
            } => {
                return self
                    .fetch_models_from_chatgpt_api(
                        http_client,
                        model_caps,
                        access_token,
                        chatgpt_account_id,
                    )
                    .await;
            }
            _ => {}
        }

        self.fetch_models_from_catalog(model_caps)
    }

    fn set_model_enabled(&mut self, model_id: &str, enabled: bool) {
        set_model_enabled_impl(&mut self.enabled_models, model_id, enabled);
    }

    fn add_custom_model(&mut self, model_id: String, config: CustomModelConfig) {
        self.custom_models.insert(model_id, config);
    }

    fn remove_custom_model(&mut self, model_id: &str) -> bool {
        self.custom_models.remove(model_id).is_some()
    }

    fn custom_model_pricing(&self, model_id: &str) -> Option<ModelPricing> {
        if let Some(config) = self.custom_models.get(model_id) {
            if config.pricing.is_some() {
                return config.pricing.clone();
            }
        }
        None
    }

    async fn startup_refresh_and_sync(
        &mut self,
        http_client: &reqwest::Client,
        config_dir: &std::path::Path,
    ) -> Result<(), String> {
        if self.oauth_tokens.is_empty() || self.oauth_tokens.refresh_token.is_empty() {
            return Ok(());
        }

        if !Self::needs_refresh_on_start(self.oauth_tokens.expires_at) {
            return Ok(());
        }

        tracing::info!("OpenAI Codex: refreshing OAuth token on startup");
        let mut refreshed = match crate::providers::openai_codex_oauth::refresh_access_token(
            http_client,
            &self.oauth_tokens.refresh_token,
        )
        .await
        {
            Ok(refreshed) => refreshed,
            Err(e) if crate::providers::oauth_refresh::is_permanent_refresh_error(&e) => {
                crate::providers::oauth_refresh::mark_invalid_refresh_token(
                    "openai_codex",
                    &self.oauth_tokens.refresh_token,
                );
                tracing::warn!(
                    "OpenAI Codex: OAuth refresh token is invalid; clearing saved refresh token. Please log in again if Codex stops working: {}",
                    e
                );
                self.oauth_tokens.access_token.clear();
                self.oauth_tokens.refresh_token.clear();
                self.oauth_tokens.expires_at = 0;
                self.save_oauth_tokens_config(config_dir).await?;
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        if refreshed.openai_api_key.is_empty() {
            refreshed.openai_api_key = self.oauth_tokens.openai_api_key.clone();
        }
        if refreshed.chatgpt_account_id.is_empty() {
            refreshed.chatgpt_account_id = self.oauth_tokens.chatgpt_account_id.clone();
        }
        if refreshed.api_key_exchange_error.is_empty() {
            refreshed.api_key_exchange_error = self.oauth_tokens.api_key_exchange_error.clone();
        }

        self.oauth_tokens = refreshed;
        self.save_oauth_tokens_config(config_dir).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use super::OpenAICodexProvider;
    use crate::caps::model_caps::ModelCapabilities;
    use crate::providers::openai_codex_oauth::OAuthTokens;
    use crate::providers::traits::{CustomModelConfig, ModelPricing, ModelSource, ProviderTrait};

    fn provider_with_api_key(api_key: &str) -> OpenAICodexProvider {
        OpenAICodexProvider {
            oauth_tokens: OAuthTokens {
                openai_api_key: api_key.to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn provider_with_oauth(access_token: &str, chatgpt_account_id: &str) -> OpenAICodexProvider {
        OpenAICodexProvider {
            oauth_tokens: OAuthTokens {
                access_token: access_token.to_string(),
                chatgpt_account_id: chatgpt_account_id.to_string(),
                expires_at: i64::MAX,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn codex_caps(n_ctx: usize) -> ModelCapabilities {
        ModelCapabilities {
            n_ctx,
            max_output_tokens: 16_384,
            supports_tools: true,
            supports_parallel_tools: true,
            supports_vision: true,
            reasoning_effort_options: Some(vec![
                "low".to_string(),
                "medium".to_string(),
                "high".to_string(),
            ]),
            pricing: Some(ModelPricing {
                prompt: 3.0,
                generated: 12.0,
                cache_read: Some(1.5),
                cache_creation: Some(4.5),
            }),
            ..Default::default()
        }
    }

    fn caps_map() -> HashMap<String, ModelCapabilities> {
        HashMap::from([
            ("openai/gpt-5.6-codex".to_string(), codex_caps(256_000)),
            ("openai/gpt-4o".to_string(), codex_caps(128_000)),
            (
                "github-copilot/gpt-5.7-codex".to_string(),
                codex_caps(128_000),
            ),
        ])
    }

    #[test]
    fn model_source_api_when_platform_key_present() {
        let p = provider_with_api_key("sk-test");
        assert_eq!(p.model_source(), ModelSource::Api);
    }

    #[test]
    fn model_source_model_caps_when_oauth_only() {
        let p = provider_with_oauth("tok", "acct-123");
        assert_eq!(p.model_source(), ModelSource::ModelCaps);
    }

    #[test]
    fn is_codex_model_matches_codex_named_models_only() {
        assert!(super::is_codex_model("gpt-5.3-codex"));
        assert!(super::is_codex_model("GPT-5.3-CODEX"));
        assert!(!super::is_codex_model("gpt-5.4"));
        assert!(!super::is_codex_model("gpt-5.5"));
        assert!(!super::is_codex_model("gpt-4o"));
    }

    #[test]
    fn model_filter_regex_matches_codex_named_only() {
        let p = provider_with_api_key("sk-test");
        let pattern = p.model_filter_regex().expect("filter regex must be set");
        let re = regex::Regex::new(pattern).unwrap();

        assert!(re.is_match("gpt-5.3-codex"));
        assert!(re.is_match("GPT-5.3-CODEX"));
        assert!(!re.is_match("gpt-5.4"));
        assert!(!re.is_match("gpt-4o"));
    }

    #[tokio::test]
    async fn no_auth_returns_empty_when_no_custom_models() {
        let p = OpenAICodexProvider::default();
        let client = reqwest::Client::new();
        let models = p.fetch_available_models(&client, &HashMap::new()).await;
        assert!(models.is_empty());
    }

    #[test]
    fn catalog_fallback_uses_models_dev_caps_without_hardcoded_ids() {
        let p = provider_with_oauth("tok", "acct-123");
        let models = p.fetch_models_from_catalog(&caps_map());
        let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();

        assert_eq!(ids, vec!["gpt-5.6-codex"]);
        assert!(!ids.contains(&"gpt-5.3-codex"));
        assert!(!ids.contains(&"gpt-4o"));
        assert!(!ids.contains(&"gpt-5.7-codex"));
        assert_eq!(models[0].n_ctx, 256_000);
    }

    #[test]
    fn custom_models_still_appear_and_override_pricing() {
        let mut p = provider_with_oauth("tok", "acct-123");
        p.enabled_models = vec!["gpt-5.6-codex".to_string(), "my-custom".to_string()];
        p.custom_models.insert(
            "gpt-5.6-codex".to_string(),
            CustomModelConfig {
                pricing: Some(ModelPricing {
                    prompt: 101.0,
                    generated: 202.0,
                    cache_read: Some(303.0),
                    cache_creation: Some(404.0),
                }),
                ..Default::default()
            },
        );
        p.custom_models.insert(
            "my-custom".to_string(),
            CustomModelConfig {
                n_ctx: Some(4096),
                supports_tools: Some(true),
                pricing: Some(ModelPricing {
                    prompt: 1.0,
                    generated: 2.0,
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        let models = p.fetch_models_from_catalog(&caps_map());
        let codex = models.iter().find(|m| m.id == "gpt-5.6-codex").unwrap();
        let custom = models.iter().find(|m| m.id == "my-custom").unwrap();

        assert!(codex.enabled);
        assert_eq!(codex.pricing.as_ref().unwrap().prompt, 101.0);
        assert!(custom.enabled);
        assert!(custom.is_custom);
        assert_eq!(custom.pricing.as_ref().unwrap().generated, 2.0);
    }

    #[test]
    fn chatgpt_backend_headers_include_session_originator_and_account() {
        let mut p = provider_with_oauth("tok", "acct-123");
        p.session_id = "session-test".to_string();

        let headers = p.chatgpt_backend_headers("acct-123");

        assert_eq!(
            headers.get("originator").map(String::as_str),
            Some("refact-lsp")
        );
        assert_eq!(
            headers.get("session_id").map(String::as_str),
            Some("session-test")
        );
        assert_eq!(
            headers.get("chatgpt-account-id").map(String::as_str),
            Some("acct-123")
        );
        assert_eq!(
            headers.get("OpenAI-Beta").map(String::as_str),
            Some("responses=experimental")
        );
    }

    #[test]
    fn runtime_headers_include_chatgpt_backend_metadata() {
        let mut p = provider_with_oauth("tok", "acct-123");
        p.session_id = "session-test".to_string();
        p.enabled_models = vec!["gpt-5.6-codex".to_string()];

        let runtime = p.build_runtime().unwrap();

        assert!(runtime.enabled);
        assert_eq!(
            runtime.chat_endpoint,
            "https://chatgpt.com/backend-api/codex/responses"
        );
        assert_eq!(runtime.api_key, "tok");
        assert_eq!(
            runtime.extra_headers.get("originator").map(String::as_str),
            Some("refact-lsp")
        );
        assert_eq!(
            runtime.extra_headers.get("session_id").map(String::as_str),
            Some("session-test")
        );
        assert_eq!(
            runtime
                .extra_headers
                .get("chatgpt-account-id")
                .map(String::as_str),
            Some("acct-123")
        );
    }

    #[test]
    fn forced_refresh_decision_is_bounded_to_one_auth_rejection() {
        assert!(OpenAICodexProvider::should_force_refresh_for_status(
            reqwest::StatusCode::UNAUTHORIZED,
            "refresh",
            false,
        ));
        assert!(OpenAICodexProvider::should_force_refresh_for_status(
            reqwest::StatusCode::FORBIDDEN,
            "refresh",
            false,
        ));
        assert!(!OpenAICodexProvider::should_force_refresh_for_status(
            reqwest::StatusCode::UNAUTHORIZED,
            "refresh",
            true,
        ));
        assert!(!OpenAICodexProvider::should_force_refresh_for_status(
            reqwest::StatusCode::UNAUTHORIZED,
            "",
            false,
        ));
        assert!(!OpenAICodexProvider::should_force_refresh_for_status(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            "refresh",
            false,
        ));
    }

    #[test]
    fn refreshed_tokens_preserve_codex_account_context() {
        let p = OpenAICodexProvider {
            oauth_tokens: OAuthTokens {
                access_token: "old".to_string(),
                refresh_token: "refresh-old".to_string(),
                openai_api_key: "sk-old".to_string(),
                chatgpt_account_id: "acct-123".to_string(),
                api_key_exchange_error: "no-platform-key".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let merged = p.merge_refreshed_tokens(OAuthTokens {
            access_token: "new".to_string(),
            refresh_token: "refresh-new".to_string(),
            expires_at: 42,
            ..Default::default()
        });

        assert_eq!(merged.access_token, "new");
        assert_eq!(merged.refresh_token, "refresh-new");
        assert_eq!(merged.openai_api_key, "sk-old");
        assert_eq!(merged.chatgpt_account_id, "acct-123");
        assert_eq!(merged.api_key_exchange_error, "no-platform-key");
    }

    #[test]
    fn invalid_refresh_token_clearing_preserves_platform_key() {
        let mut p = OpenAICodexProvider {
            oauth_tokens: OAuthTokens {
                access_token: "old".to_string(),
                refresh_token: "refresh-old".to_string(),
                expires_at: 123,
                openai_api_key: "sk-old".to_string(),
                chatgpt_account_id: "acct-123".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        p.clear_tokens_after_permanent_refresh_error();

        assert!(p.oauth_tokens.access_token.is_empty());
        assert!(p.oauth_tokens.refresh_token.is_empty());
        assert_eq!(p.oauth_tokens.expires_at, 0);
        assert_eq!(p.oauth_tokens.openai_api_key, "sk-old");
        assert_eq!(p.oauth_tokens.chatgpt_account_id, "acct-123");
    }

    #[test]
    fn wham_usage_parser_handles_plan_windows_and_credits() {
        let usage = OpenAICodexProvider::parse_usage_payload(&json!({
            "data": {
                "plan_type": "plus",
                "rate_limit": {
                    "primary_window": {
                        "used_percent": 12.5,
                        "limit_window_seconds": 18_000,
                        "reset_at": 1_700_000_000
                    },
                    "secondary_window": {
                        "used_percent": "99.5",
                        "limit_window_seconds": "604800",
                        "reset_at": "2026-01-01T00:00:00Z"
                    }
                },
                "code_review_rate_limit": {
                    "primary_window": { "usedPercent": 100.0 }
                },
                "credits": {
                    "balance": "12.75",
                    "unlimited": false,
                    "has_credits": true,
                    "granted": 20,
                    "used": "7.25",
                    "expires_at": 1_700_000_000_000i64
                }
            }
        }));

        assert_eq!(usage.plan_type.as_deref(), Some("plus"));
        let rate_limit = usage.rate_limit.unwrap();
        assert!(!rate_limit.limit_reached);
        let primary = rate_limit.primary_window.unwrap();
        assert_eq!(primary.used_percent, 12.5);
        assert_eq!(primary.limit_window_seconds, Some(18_000));
        assert!(primary.reset_at.unwrap().starts_with("2023-11-14T"));
        let secondary = rate_limit.secondary_window.unwrap();
        assert_eq!(secondary.used_percent, 99.5);
        assert_eq!(secondary.reset_at.as_deref(), Some("2026-01-01T00:00:00Z"));
        assert!(usage.code_review_rate_limit.unwrap().limit_reached);
        let credits = usage.credits.unwrap();
        assert_eq!(credits.balance, 12.75);
        assert_eq!(credits.granted, Some(20.0));
        assert_eq!(credits.used, Some(7.25));
        assert!(credits.reset_at.unwrap().starts_with("2023-11-14T"));
    }
}
