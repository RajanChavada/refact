use std::sync::Arc;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::buddy::observers::{BuddyObserver, ObserverContext};
use crate::buddy::settings::BuddySettings;
use crate::buddy::types::{BuddyFact, BuddyFactKind};
use crate::caps::DefaultModels;
use crate::global_context::GlobalContext;

pub struct ProviderHealthObserver;

pub fn detect_provider_health_facts(
    defaults: &DefaultModels,
    available_models: &[String],
    now: DateTime<Utc>,
) -> Vec<BuddyFact> {
    let mut facts = vec![];
    let fields = [
        (
            "chat_default_model",
            defaults.chat_default_model.as_str(),
            "chat_model",
        ),
        (
            "chat_buddy_model",
            defaults.chat_buddy_model.as_str(),
            "chat_buddy_model",
        ),
        (
            "chat_thinking_model",
            defaults.chat_thinking_model.as_str(),
            "chat_thinking_model",
        ),
        (
            "chat_light_model",
            defaults.chat_light_model.as_str(),
            "chat_light_model",
        ),
        (
            "completion_default_model",
            defaults.completion_default_model.as_str(),
            "completion_model",
        ),
    ];
    for (field_name, model_id, payload_field) in &fields {
        if model_id.is_empty() {
            facts.push(BuddyFact {
                kind: BuddyFactKind::DefaultModelMissing,
                key: format!("provider:default_missing:{}", field_name),
                source: "provider_health",
                payload: serde_json::json!({ "field": payload_field, "model_id": null }),
                seen_at: now,
                confidence: 0.95,
            });
        } else if !available_models
            .iter()
            .any(|available| available == model_id)
        {
            facts.push(BuddyFact {
                kind: BuddyFactKind::BrokenModelReference,
                key: format!("provider:broken_ref:{}", field_name),
                source: "provider_health",
                payload: serde_json::json!({ "field": payload_field, "model_id": model_id }),
                seen_at: now,
                confidence: 0.9,
            });
        }
    }
    facts
}

#[async_trait::async_trait]
impl BuddyObserver for ProviderHealthObserver {
    fn id(&self) -> &'static str {
        "provider_health"
    }

    fn cadence_seconds(&self) -> u64 {
        300
    }

    fn requires_setting(&self, settings: &BuddySettings) -> bool {
        settings.observers.provider_health && settings.proactive_enabled
    }

    async fn observe(
        &self,
        gcx: Arc<RwLock<GlobalContext>>,
        _ctx: &ObserverContext,
    ) -> Vec<BuddyFact> {
        let gcx_read = gcx.read().await;
        let caps = match &gcx_read.caps {
            Some(c) => c.clone(),
            None => return vec![],
        };
        let mut available: Vec<String> = caps.chat_models.keys().cloned().collect();
        available.extend(caps.completion_models.keys().cloned());
        detect_provider_health_facts(&caps.defaults, &available, Utc::now())
    }
}
