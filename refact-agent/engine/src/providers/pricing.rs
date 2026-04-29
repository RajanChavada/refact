use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock as ARwLock;

use crate::call_validation::{ChatUsage, MeteringUsd};
use crate::caps::model_caps::{resolve_model_caps, ModelCapabilities};
use crate::global_context::GlobalContext;
use crate::providers::traits::ModelPricing;

pub fn compute_cost(usage: &ChatUsage, pricing: &ModelPricing) -> Option<MeteringUsd> {
    if !pricing.is_valid() {
        return None;
    }

    let prompt_usd = (usage.prompt_tokens as f64) * pricing.prompt / 1_000_000.0;
    let generated_usd = (usage.completion_tokens as f64) * pricing.generated / 1_000_000.0;

    let cache_read_usd = match (usage.cache_read_tokens, pricing.cache_read) {
        (Some(tokens), Some(rate)) => Some((tokens as f64) * rate / 1_000_000.0),
        _ => None,
    };

    let cache_creation_usd = match (usage.cache_creation_tokens, pricing.cache_creation) {
        (Some(tokens), Some(rate)) => Some((tokens as f64) * rate / 1_000_000.0),
        _ => None,
    };

    let total_usd = prompt_usd
        + generated_usd
        + cache_read_usd.unwrap_or(0.0)
        + cache_creation_usd.unwrap_or(0.0);

    Some(MeteringUsd {
        prompt_usd,
        generated_usd,
        cache_read_usd,
        cache_creation_usd,
        total_usd,
    })
}

pub fn model_caps_pricing(
    model_caps: &HashMap<String, ModelCapabilities>,
    model_id: &str,
) -> Option<ModelPricing> {
    if let Some(pricing) =
        resolve_model_caps(model_caps, model_id).and_then(|resolved| resolved.caps.pricing)
    {
        return Some(pricing);
    }

    let Some((provider_name, bare_model_id)) = model_id.split_once('/') else {
        return None;
    };
    for provider_alias in pricing_provider_aliases(provider_name) {
        let qualified = format!("{provider_alias}/{bare_model_id}");
        if let Some(pricing) =
            resolve_model_caps(model_caps, &qualified).and_then(|resolved| resolved.caps.pricing)
        {
            return Some(pricing);
        }
    }

    resolve_model_caps(model_caps, bare_model_id).and_then(|resolved| resolved.caps.pricing)
}

fn pricing_provider_aliases(provider_name: &str) -> Vec<String> {
    let mut aliases = vec![provider_name.replace('_', "-")];
    for suffix in ["_responses", "-responses"] {
        if let Some(stripped) = provider_name.strip_suffix(suffix) {
            aliases.push(stripped.to_string());
            aliases.push(stripped.replace('_', "-"));
        }
    }
    if provider_name == "google_gemini" {
        aliases.push("google".to_string());
    }
    aliases.sort();
    aliases.dedup();
    aliases
}

pub async fn lookup_model_pricing(
    gcx: &Arc<ARwLock<GlobalContext>>,
    model_id: &str,
) -> Option<ModelPricing> {
    if let Some(custom_pricing) = lookup_provider_custom_pricing(gcx, model_id).await {
        return Some(custom_pricing);
    }

    let caps = crate::global_context::try_load_caps_quickly_if_not_present(gcx.clone(), 0)
        .await
        .ok()?;
    model_caps_pricing(&caps.model_caps, model_id)
}

async fn lookup_provider_custom_pricing(
    gcx: &Arc<ARwLock<GlobalContext>>,
    model_id: &str,
) -> Option<ModelPricing> {
    let (provider_name, model_name) = model_id.split_once('/')?;
    let gcx_locked = gcx.read().await;
    let registry = gcx_locked.providers.read().await;
    registry
        .get(provider_name)
        .and_then(|provider| provider.model_pricing(model_name))
}
