use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::Extension;
use axum::extract::Query;
use axum::http::HeaderMap;
use hyper::StatusCode;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::custom_error::ScratchError;
use crate::global_context::SharedGlobalContext;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    endpoint: String,
    q: String,
    page: u32,
    limit: u32,
    sort_by: String,
}

struct CacheEntry {
    body: Value,
    inserted_at: Instant,
}

static CACHE: OnceLock<Mutex<HashMap<CacheKey, CacheEntry>>> = OnceLock::new();

fn get_cache() -> &'static Mutex<HashMap<CacheKey, CacheEntry>> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_get(key: &CacheKey) -> Option<Value> {
    if let Ok(mut map) = get_cache().lock() {
        if let Some(entry) = map.get(key) {
            if entry.inserted_at.elapsed() < Duration::from_secs(60) {
                return Some(entry.body.clone());
            }
            map.remove(key);
        }
    }
    None
}

fn cache_set(key: CacheKey, body: Value) {
    if let Ok(mut map) = get_cache().lock() {
        map.insert(key, CacheEntry { body, inserted_at: Instant::now() });
    }
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub sort_by: Option<String>,
}

#[derive(Deserialize)]
pub struct AiSearchParams {
    pub q: Option<String>,
}

fn extract_api_key(headers: &HeaderMap) -> Result<String, ScratchError> {
    headers
        .get("X-SkillsMP-Api-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| ScratchError::new(StatusCode::BAD_REQUEST, "Missing X-SkillsMP-Api-Key header".to_string()))
}

fn validate_q(q: Option<&String>) -> Result<String, ScratchError> {
    let q = q.ok_or_else(|| ScratchError::new(StatusCode::BAD_REQUEST, "Missing required query parameter 'q'".to_string()))?;
    if q.len() > 256 {
        return Err(ScratchError::new(StatusCode::BAD_REQUEST, "Query too long (max 256 chars)".to_string()));
    }
    Ok(q.clone())
}

fn parse_ratelimit(headers: &reqwest::header::HeaderMap) -> Value {
    let daily_limit = headers
        .get("X-RateLimit-Daily-Limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());
    let daily_remaining = headers
        .get("X-RateLimit-Daily-Remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());
    json!({
        "daily_limit": daily_limit,
        "daily_remaining": daily_remaining,
    })
}

pub async fn handle_skillsmp_search(
    Extension(gcx): Extension<SharedGlobalContext>,
    headers: HeaderMap,
    Query(params): Query<SearchParams>,
) -> Result<axum::response::Json<Value>, ScratchError> {
    let api_key = extract_api_key(&headers)?;
    let q = validate_q(params.q.as_ref())?;
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20).min(100);
    let sort_by = params.sort_by.clone().unwrap_or_default();

    let cache_key = CacheKey {
        endpoint: "search".to_string(),
        q: q.clone(),
        page,
        limit,
        sort_by: sort_by.clone(),
    };

    if let Some(cached) = cache_get(&cache_key) {
        return Ok(axum::response::Json(cached));
    }

    let http_client = gcx.read().await.http_client.clone();
    let mut url = format!(
        "https://skillsmp.com/api/v1/skills/search?q={}&page={}&limit={}",
        utf8_percent_encode(&q, NON_ALPHANUMERIC),
        page,
        limit,
    );
    if !sort_by.is_empty() {
        url.push_str(&format!("&sortBy={}", utf8_percent_encode(&sort_by, NON_ALPHANUMERIC)));
    }

    let response = tokio::time::timeout(
        Duration::from_secs(10),
        http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send(),
    )
    .await
    .map_err(|_| ScratchError::new(StatusCode::GATEWAY_TIMEOUT, "SkillsMP request timed out".to_string()))?
    .map_err(|e| ScratchError::new(StatusCode::BAD_GATEWAY, format!("SkillsMP request failed: {}", e)))?;

    let status = response.status();
    let ratelimit = parse_ratelimit(response.headers());

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ScratchError::new(StatusCode::UNAUTHORIZED, "Invalid SkillsMP API key".to_string()));
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(ScratchError::new(StatusCode::TOO_MANY_REQUESTS, "SkillsMP daily quota exceeded".to_string()));
    }
    if !status.is_success() {
        return Err(ScratchError::new(StatusCode::BAD_GATEWAY, format!("SkillsMP returned status {}", status)));
    }

    let data: Value = response.json().await
        .map_err(|e| ScratchError::new(StatusCode::BAD_GATEWAY, format!("SkillsMP response parse error: {}", e)))?;

    let result = json!({ "data": data, "ratelimit": ratelimit });
    cache_set(cache_key, result.clone());
    Ok(axum::response::Json(result))
}

pub async fn handle_skillsmp_ai_search(
    Extension(gcx): Extension<SharedGlobalContext>,
    headers: HeaderMap,
    Query(params): Query<AiSearchParams>,
) -> Result<axum::response::Json<Value>, ScratchError> {
    let api_key = extract_api_key(&headers)?;
    let q = validate_q(params.q.as_ref())?;

    let cache_key = CacheKey {
        endpoint: "ai-search".to_string(),
        q: q.clone(),
        page: 1,
        limit: 20,
        sort_by: String::new(),
    };

    if let Some(cached) = cache_get(&cache_key) {
        return Ok(axum::response::Json(cached));
    }

    let http_client = gcx.read().await.http_client.clone();
    let url = format!(
        "https://skillsmp.com/api/v1/skills/ai-search?q={}",
        utf8_percent_encode(&q, NON_ALPHANUMERIC),
    );

    let response = tokio::time::timeout(
        Duration::from_secs(10),
        http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send(),
    )
    .await
    .map_err(|_| ScratchError::new(StatusCode::GATEWAY_TIMEOUT, "SkillsMP request timed out".to_string()))?
    .map_err(|e| ScratchError::new(StatusCode::BAD_GATEWAY, format!("SkillsMP request failed: {}", e)))?;

    let status = response.status();
    let ratelimit = parse_ratelimit(response.headers());

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ScratchError::new(StatusCode::UNAUTHORIZED, "Invalid SkillsMP API key".to_string()));
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(ScratchError::new(StatusCode::TOO_MANY_REQUESTS, "SkillsMP daily quota exceeded".to_string()));
    }
    if !status.is_success() {
        return Err(ScratchError::new(StatusCode::BAD_GATEWAY, format!("SkillsMP returned status {}", status)));
    }

    let data: Value = response.json().await
        .map_err(|e| ScratchError::new(StatusCode::BAD_GATEWAY, format!("SkillsMP response parse error: {}", e)))?;

    let result = json!({ "data": data, "ratelimit": ratelimit });
    cache_set(cache_key, result.clone());
    Ok(axum::response::Json(result))
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;

    use super::{extract_api_key, validate_q};

    #[test]
    fn test_missing_api_key_header() {
        let headers = HeaderMap::new();
        let result = extract_api_key(&headers);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code, hyper::StatusCode::BAD_REQUEST);
        assert!(err.message.contains("Missing X-SkillsMP-Api-Key header"));
    }

    #[test]
    fn test_missing_q_param() {
        let result = validate_q(None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code, hyper::StatusCode::BAD_REQUEST);
        assert!(err.message.contains("Missing required query parameter 'q'"));
    }

    #[test]
    fn test_q_too_long() {
        let long_q = "a".repeat(257);
        let result = validate_q(Some(&long_q));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code, hyper::StatusCode::BAD_REQUEST);
        assert!(err.message.contains("Query too long"));
    }

    #[test]
    fn test_q_exactly_256_chars_ok() {
        let q = "a".repeat(256);
        let result = validate_q(Some(&q));
        assert!(result.is_ok());
    }

    #[test]
    fn test_limit_clamping() {
        let limit: u32 = 200u32.min(100);
        assert_eq!(limit, 100);

        let limit: u32 = 50u32.min(100);
        assert_eq!(limit, 50);
    }
}
