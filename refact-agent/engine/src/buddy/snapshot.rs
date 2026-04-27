use serde::{Deserialize, Serialize};
use super::diagnostics::DiagnosticContext;
use super::settings::BuddySettings;
use super::types::{BuddyRuntimeEvent, BuddySpeechItem, BuddyState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuddySnapshot {
    pub state: BuddyState,
    pub settings: BuddySettings,
    pub enabled: bool,
    #[serde(default)]
    pub recent_diagnostics: Vec<DiagnosticContext>,
    pub runtime_queue: Vec<BuddyRuntimeEvent>,
    pub now_playing: Option<BuddyRuntimeEvent>,
    pub active_speech: Option<BuddySpeechItem>,
}
