use std::path::Path;
use serde::{Serialize, Deserialize};
use tokio::fs;
use tracing::warn;

pub const MAX_PALETTE_INDEX: usize = 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuddySettings {
    pub enabled: bool,
    pub auto_diagnostics: bool,
    pub auto_issue_creation: bool,
    pub personality_prompt: Option<String>,
    #[serde(default = "default_true")]
    pub proactive_enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for BuddySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_diagnostics: true,
            auto_issue_creation: false,
            personality_prompt: None,
            proactive_enabled: true,
        }
    }
}

pub async fn load_settings(project_root: &Path) -> BuddySettings {
    let path = project_root.join(".refact/buddy/settings.json");
    match fs::read_to_string(&path).await {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to parse buddy settings: {}, using defaults", e);
                BuddySettings::default()
            }
        },
        Err(_) => BuddySettings::default(),
    }
}

pub async fn save_settings(project_root: &Path, settings: &BuddySettings) -> Result<(), String> {
    let path = project_root.join(".refact/buddy/settings.json");
    super::storage::atomic_write_json(&path, settings).await
}
