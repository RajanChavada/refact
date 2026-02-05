use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as ARwLock;
use tracing::{info, warn};

use crate::global_context::GlobalContext;

const MODEL_CAPS_URL: &str = "https://www.smallcloud.ai/v1/model-capabilities";
const CACHE_FILENAME: &str = "model-capabilities.json";
const CACHE_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapsSource {
    Registry,
    Finetune,
    Custom,
}

impl Default for ModelCapsSource {
    fn default() -> Self {
        Self::Registry
    }
}

#[derive(Debug, Clone)]
pub struct CanonicalNameParts {
    pub original: String,
    pub provider_stripped: String,
    pub base_model: String,
    pub is_finetune: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedCaps {
    pub caps: ModelCapabilities,
    pub source: ModelCapsSource,
    pub matched_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningType {
    None,
    Openai,
    Anthropic,
    Deepseek,
    Xai,
    Qwen,
}

impl Default for ReasoningType {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CachingType {
    None,
    Auto,
    Explicit,
}

impl Default for CachingType {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelCapabilities {
    pub n_ctx: usize,
    pub max_output_tokens: usize,
    #[serde(default)]
    pub supports_tools: bool,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub supports_video: bool,
    #[serde(default)]
    pub supports_audio: bool,
    #[serde(default)]
    pub supports_pdf: bool,
    #[serde(default)]
    pub supports_clicks: bool,
    #[serde(default = "default_true")]
    pub supports_temperature: bool,
    #[serde(default = "default_true")]
    pub supports_streaming: bool,
    #[serde(default)]
    pub reasoning: ReasoningType,
    #[serde(default)]
    pub supports_reasoning_effort: bool,
    #[serde(default)]
    pub caching: CachingType,
    #[serde(default)]
    pub tokenizer: String,
    #[serde(default)]
    pub default_temperature: Option<f32>,
    #[serde(default)]
    pub default_max_tokens: Option<usize>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedModelCaps {
    pub fetched_at: u64,
    pub models: HashMap<String, ModelCapabilities>,
}

impl CachedModelCaps {
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now - self.fetched_at > CACHE_MAX_AGE.as_secs()
    }
}

fn get_cache_path() -> PathBuf {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("refact");
    std::fs::create_dir_all(&cache_dir).ok();
    cache_dir.join(CACHE_FILENAME)
}

pub fn load_cached_model_caps() -> Option<CachedModelCaps> {
    let cache_path = get_cache_path();
    if !cache_path.exists() {
        return None;
    }

    match std::fs::read_to_string(&cache_path) {
        Ok(content) => match serde_json::from_str::<CachedModelCaps>(&content) {
            Ok(cached) => {
                info!("Loaded model capabilities from cache: {} models", cached.models.len());
                Some(cached)
            }
            Err(e) => {
                warn!("Failed to parse cached model capabilities: {}", e);
                None
            }
        },
        Err(e) => {
            warn!("Failed to read cached model capabilities: {}", e);
            None
        }
    }
}

pub fn save_cached_model_caps(caps: &CachedModelCaps) -> Result<(), String> {
    let cache_path = get_cache_path();
    let content = serde_json::to_string_pretty(caps)
        .map_err(|e| format!("Failed to serialize model capabilities: {}", e))?;
    std::fs::write(&cache_path, content)
        .map_err(|e| format!("Failed to write model capabilities cache: {}", e))?;
    info!("Saved model capabilities to cache: {}", cache_path.display());
    Ok(())
}

pub async fn fetch_model_caps_from_server(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<HashMap<String, ModelCapabilities>, String> {
    let http_client = gcx.read().await.http_client.clone();

    info!("Fetching model capabilities from {}", MODEL_CAPS_URL);

    let response = http_client
        .get(MODEL_CAPS_URL)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch model capabilities: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("Server returned status {}", status));
    }

    let models: HashMap<String, ModelCapabilities> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse model capabilities response: {}", e))?;

    info!("Fetched {} model capabilities from server", models.len());
    Ok(models)
}

pub async fn get_model_caps(
    gcx: Arc<ARwLock<GlobalContext>>,
    force_refresh: bool,
) -> Result<HashMap<String, ModelCapabilities>, String> {
    if !force_refresh {
        if let Some(cached) = load_cached_model_caps() {
            if !cached.is_expired() {
                return Ok(cached.models);
            }
            info!("Cached model capabilities expired, fetching fresh data");
        }
    }

    match fetch_model_caps_from_server(gcx).await {
        Ok(models) => {
            let cached = CachedModelCaps {
                fetched_at: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                models: models.clone(),
            };
            if let Err(e) = save_cached_model_caps(&cached) {
                warn!("Failed to save model capabilities cache: {}", e);
            }
            Ok(models)
        }
        Err(e) => {
            warn!("Failed to fetch model capabilities from server: {}", e);
            if let Some(cached) = load_cached_model_caps() {
                warn!("Using expired cached model capabilities as fallback");
                return Ok(cached.models);
            }
            Err(e)
        }
    }
}

pub fn is_model_supported(caps: &HashMap<String, ModelCapabilities>, model_name: &str) -> bool {
    resolve_model_caps(caps, model_name).is_some()
}

pub fn canonicalize_model_name(model_id: &str) -> CanonicalNameParts {
    let provider_stripped = model_id
        .split('/')
        .last()
        .unwrap_or(model_id)
        .to_string();

    let (base_model, is_finetune) = if let Some(colon_pos) = provider_stripped.find(':') {
        let base = provider_stripped[..colon_pos].to_string();
        let suffix = &provider_stripped[colon_pos + 1..];
        let is_ft = suffix.starts_with("ft-") || suffix.starts_with("ft_");
        (base, is_ft)
    } else {
        (provider_stripped.clone(), false)
    };

    CanonicalNameParts {
        original: model_id.to_string(),
        provider_stripped,
        base_model,
        is_finetune,
    }
}

fn matches_pattern(pattern: &str, name: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == name;
    }

    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        return name.starts_with(prefix);
    }

    if pattern.starts_with('*') {
        let suffix = &pattern[1..];
        return name.ends_with(suffix);
    }

    if let Some(star_pos) = pattern.find('*') {
        let prefix = &pattern[..star_pos];
        let suffix = &pattern[star_pos + 1..];
        return name.starts_with(prefix) && name.ends_with(suffix);
    }

    false
}

fn pattern_specificity(pattern: &str) -> usize {
    pattern.chars().filter(|c| *c != '*').count()
}

pub fn resolve_model_caps(
    caps: &HashMap<String, ModelCapabilities>,
    model_name: &str,
) -> Option<ResolvedCaps> {
    let canonical = canonicalize_model_name(model_name);

    let names_to_try = [
        &canonical.original,
        &canonical.provider_stripped,
        &canonical.base_model,
    ];

    for name in &names_to_try {
        if let Some(model_caps) = caps.get(*name) {
            let source = if canonical.is_finetune && *name == &canonical.base_model {
                ModelCapsSource::Finetune
            } else {
                ModelCapsSource::Registry
            };
            return Some(ResolvedCaps {
                caps: model_caps.clone(),
                source,
                matched_key: (*name).clone(),
            });
        }
    }

    let mut best_match: Option<(&str, &ModelCapabilities, usize)> = None;

    for (pattern, model_caps) in caps.iter() {
        if !pattern.contains('*') {
            continue;
        }

        for name in &names_to_try {
            if matches_pattern(pattern, name) {
                let specificity = pattern_specificity(pattern);
                if best_match.is_none() || specificity > best_match.unwrap().2 {
                    best_match = Some((pattern, model_caps, specificity));
                } else if specificity == best_match.unwrap().2 && pattern.as_str() < best_match.unwrap().0 {
                    best_match = Some((pattern, model_caps, specificity));
                }
            }
        }
    }

    best_match.map(|(matched_key, model_caps, _)| {
        let source = if canonical.is_finetune {
            ModelCapsSource::Finetune
        } else {
            ModelCapsSource::Registry
        };
        ResolvedCaps {
            caps: model_caps.clone(),
            source,
            matched_key: matched_key.to_string(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_capability_lookup() {
        let mut caps = HashMap::new();
        caps.insert("gpt-4o".to_string(), ModelCapabilities {
            n_ctx: 128000,
            max_output_tokens: 16384,
            supports_tools: true,
            supports_vision: true,
            ..Default::default()
        });
        caps.insert("claude-3-5-sonnet".to_string(), ModelCapabilities {
            n_ctx: 200000,
            max_output_tokens: 8192,
            supports_tools: true,
            supports_vision: true,
            supports_pdf: true,
            ..Default::default()
        });

        assert!(resolve_model_caps(&caps, "gpt-4o").is_some());
        assert!(resolve_model_caps(&caps, "openai/gpt-4o").is_some());
        assert!(resolve_model_caps(&caps, "gpt-4o:v2").is_some());
        assert!(resolve_model_caps(&caps, "claude-3-5-sonnet").is_some());
        assert!(resolve_model_caps(&caps, "unknown-model").is_none());
    }

    #[test]
    fn test_canonicalize_model_name() {
        let parts = canonicalize_model_name("openai/gpt-4o");
        assert_eq!(parts.provider_stripped, "gpt-4o");
        assert_eq!(parts.base_model, "gpt-4o");
        assert!(!parts.is_finetune);

        let parts = canonicalize_model_name("gpt-4o:ft-abc123");
        assert_eq!(parts.provider_stripped, "gpt-4o:ft-abc123");
        assert_eq!(parts.base_model, "gpt-4o");
        assert!(parts.is_finetune);

        let parts = canonicalize_model_name("anthropic/claude-3-5-sonnet:ft-xyz");
        assert_eq!(parts.provider_stripped, "claude-3-5-sonnet:ft-xyz");
        assert_eq!(parts.base_model, "claude-3-5-sonnet");
        assert!(parts.is_finetune);
    }

    #[test]
    fn test_pattern_matching() {
        let mut caps = HashMap::new();
        caps.insert("claude-3-7-sonnet*".to_string(), ModelCapabilities {
            n_ctx: 200000,
            max_output_tokens: 16384,
            supports_tools: true,
            ..Default::default()
        });
        caps.insert("gpt-4*".to_string(), ModelCapabilities {
            n_ctx: 128000,
            max_output_tokens: 8192,
            supports_tools: true,
            ..Default::default()
        });

        let resolved = resolve_model_caps(&caps, "claude-3-7-sonnet-latest").unwrap();
        assert_eq!(resolved.matched_key, "claude-3-7-sonnet*");
        assert_eq!(resolved.caps.n_ctx, 200000);

        let resolved = resolve_model_caps(&caps, "gpt-4o").unwrap();
        assert_eq!(resolved.matched_key, "gpt-4*");
    }

    #[test]
    fn test_finetune_source() {
        let mut caps = HashMap::new();
        caps.insert("gpt-4o".to_string(), ModelCapabilities {
            n_ctx: 128000,
            max_output_tokens: 16384,
            ..Default::default()
        });

        let resolved = resolve_model_caps(&caps, "gpt-4o:ft-abc123").unwrap();
        assert_eq!(resolved.source, ModelCapsSource::Finetune);
        assert_eq!(resolved.matched_key, "gpt-4o");
    }

    #[test]
    fn test_reasoning_type_serde() {
        let json = serde_json::to_string(&ReasoningType::Openai).unwrap();
        assert_eq!(json, "\"openai\"");

        let parsed: ReasoningType = serde_json::from_str("\"anthropic\"").unwrap();
        assert_eq!(parsed, ReasoningType::Anthropic);
    }

    #[test]
    fn test_caching_type_serde() {
        let json = serde_json::to_string(&CachingType::Explicit).unwrap();
        assert_eq!(json, "\"explicit\"");

        let parsed: CachingType = serde_json::from_str("\"auto\"").unwrap();
        assert_eq!(parsed, CachingType::Auto);
    }
}
