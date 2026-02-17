use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use headless_chrome::Browser;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as AMutex, RwLock as ARwLock};
use tracing::{info, warn};
use uuid::Uuid;

use crate::chat::types::WindowBounds;
use crate::global_context::GlobalContext;
use crate::integrations::integr_chrome::ChromeTab;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecorderEvent {
    pub timestamp: f64,
    pub event_type: String,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleEntry {
    pub timestamp: f64,
    pub level: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEntry {
    pub timestamp: f64,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationSummaryEntry {
    pub timestamp: f64,
    pub summary: String,
}

pub struct BrowserRuntime {
    pub runtime_id: String,
    pub attached_chat_id: Option<String>,
    pub browser: Browser,
    pub tabs: HashMap<String, Arc<AMutex<ChromeTab>>>,
    pub profile_dir: PathBuf,
    pub window_bounds: Option<WindowBounds>,
    pub action_buffer: Vec<RecorderEvent>,
    pub console_buffer: Vec<ConsoleEntry>,
    pub network_buffer: Vec<NetworkEntry>,
    pub mutation_summary: Vec<MutationSummaryEntry>,
    pub last_send_action_cursor: usize,
    pub last_send_console_cursor: usize,
    pub last_send_network_cursor: usize,
    pub last_send_mutation_cursor: usize,
    pub last_frame_hash: Option<u64>,
    pub last_frame_data: Option<Vec<u8>>,
    pub idle_timeout: Duration,
    pub is_connected: bool,
    pub last_activity: Instant,
}

impl BrowserRuntime {
    pub fn launch(
        profile_dir: PathBuf,
        window_bounds: Option<WindowBounds>,
        chrome_path: Option<PathBuf>,
        idle_timeout: Option<Duration>,
    ) -> Result<Self, String> {
        std::fs::create_dir_all(&profile_dir)
            .map_err(|e| format!("Failed to create profile dir {:?}: {}", profile_dir, e))?;

        let window_size = window_bounds.as_ref().map(|wb| (wb.width, wb.height));
        let idle_timeout = idle_timeout.unwrap_or(Duration::from_secs(600));

        let mut launch_options = headless_chrome::LaunchOptions {
            headless: false,
            window_size,
            idle_browser_timeout: idle_timeout,
            user_data_dir: Some(profile_dir.clone()),
            ..Default::default()
        };
        if let Some(ref path) = chrome_path {
            launch_options.path = Some(path.clone());
        }

        let browser = Browser::new(launch_options).map_err(|e| e.to_string())?;
        let runtime_id = Uuid::new_v4().to_string();

        info!("BrowserRuntime {} launched with profile {:?}", runtime_id, profile_dir);

        Ok(Self {
            runtime_id,
            attached_chat_id: None,
            browser,
            tabs: HashMap::new(),
            profile_dir,
            window_bounds,
            action_buffer: Vec::new(),
            console_buffer: Vec::new(),
            network_buffer: Vec::new(),
            mutation_summary: Vec::new(),
            last_send_action_cursor: 0,
            last_send_console_cursor: 0,
            last_send_network_cursor: 0,
            last_send_mutation_cursor: 0,
            last_frame_hash: None,
            last_frame_data: None,
            idle_timeout,
            is_connected: true,
            last_activity: Instant::now(),
        })
    }

    pub fn reattach(&mut self, chat_id: &str) {
        info!(
            "BrowserRuntime {} reattached from {:?} to {}",
            self.runtime_id, self.attached_chat_id, chat_id
        );
        self.attached_chat_id = Some(chat_id.to_string());
        self.last_activity = Instant::now();
    }

    pub fn detach(&mut self) {
        info!(
            "BrowserRuntime {} detached from {:?}",
            self.runtime_id, self.attached_chat_id
        );
        self.attached_chat_id = None;
    }

    pub fn check_connection(&mut self) -> bool {
        let connected = match self.browser.get_version() {
            Ok(_) => true,
            Err(_) => false,
        };
        if self.is_connected && !connected {
            warn!("BrowserRuntime {} detected browser disconnect", self.runtime_id);
        }
        self.is_connected = connected;
        connected
    }

    pub fn is_idle_expired(&self) -> bool {
        self.last_activity.elapsed() > self.idle_timeout
    }

    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn flush_action_buffer(&mut self) -> Vec<RecorderEvent> {
        let items = self.action_buffer[self.last_send_action_cursor..].to_vec();
        self.last_send_action_cursor = self.action_buffer.len();
        items
    }

    pub fn flush_console_buffer(&mut self) -> Vec<ConsoleEntry> {
        let items = self.console_buffer[self.last_send_console_cursor..].to_vec();
        self.last_send_console_cursor = self.console_buffer.len();
        items
    }

    pub fn flush_network_buffer(&mut self) -> Vec<NetworkEntry> {
        let items = self.network_buffer[self.last_send_network_cursor..].to_vec();
        self.last_send_network_cursor = self.network_buffer.len();
        items
    }

    pub fn flush_mutation_summary(&mut self) -> Vec<MutationSummaryEntry> {
        let items = self.mutation_summary[self.last_send_mutation_cursor..].to_vec();
        self.last_send_mutation_cursor = self.mutation_summary.len();
        items
    }
}

pub fn get_browser_profile_dir(
    gcx_cache_dir: &PathBuf,
    thread_id: &str,
) -> PathBuf {
    gcx_cache_dir
        .join("browser_profiles")
        .join(thread_id)
}

pub async fn get_or_create_browser_runtime(
    gcx: Arc<ARwLock<GlobalContext>>,
    runtime_id: &str,
) -> Option<Arc<AMutex<BrowserRuntime>>> {
    let gcx_locked = gcx.read().await;
    gcx_locked.browser_runtimes.get(runtime_id).cloned()
}

pub async fn register_browser_runtime(
    gcx: Arc<ARwLock<GlobalContext>>,
    runtime: BrowserRuntime,
) -> String {
    let runtime_id = runtime.runtime_id.clone();
    let arc = Arc::new(AMutex::new(runtime));
    gcx.write().await.browser_runtimes.insert(runtime_id.clone(), arc);
    runtime_id
}

pub async fn remove_browser_runtime(
    gcx: Arc<ARwLock<GlobalContext>>,
    runtime_id: &str,
) -> Option<Arc<AMutex<BrowserRuntime>>> {
    gcx.write().await.browser_runtimes.remove(runtime_id)
}

pub fn flush_buffer_since<T: Clone>(buffer: &[T], cursor: &mut usize) -> Vec<T> {
    let items = buffer[*cursor..].to_vec();
    *cursor = buffer.len();
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_get_browser_profile_dir() {
        let cache_dir = PathBuf::from("/tmp/refact-cache");
        let profile = get_browser_profile_dir(&cache_dir, "thread-abc-123");
        assert_eq!(
            profile,
            PathBuf::from("/tmp/refact-cache/browser_profiles/thread-abc-123")
        );
    }

    #[test]
    fn test_get_browser_profile_dir_different_threads() {
        let cache_dir = PathBuf::from("/home/user/.cache/refact");
        let p1 = get_browser_profile_dir(&cache_dir, "thread-1");
        let p2 = get_browser_profile_dir(&cache_dir, "thread-2");
        assert_ne!(p1, p2);
        assert!(p1.to_str().unwrap().contains("thread-1"));
        assert!(p2.to_str().unwrap().contains("thread-2"));
    }

    #[test]
    fn test_flush_buffer_since_basic() {
        let buffer = vec![
            RecorderEvent { timestamp: 1.0, event_type: "click".to_string(), details: serde_json::json!({}) },
            RecorderEvent { timestamp: 2.0, event_type: "type".to_string(), details: serde_json::json!({}) },
        ];
        let mut cursor = 0usize;

        let flushed = flush_buffer_since(&buffer, &mut cursor);
        assert_eq!(flushed.len(), 2);
        assert_eq!(cursor, 2);

        let flushed2 = flush_buffer_since(&buffer, &mut cursor);
        assert_eq!(flushed2.len(), 0);
        assert_eq!(cursor, 2);
    }

    #[test]
    fn test_flush_buffer_since_incremental() {
        let mut buffer = vec![
            ConsoleEntry { timestamp: 1.0, level: "log".to_string(), text: "hello".to_string() },
        ];
        let mut cursor = 0usize;

        let flushed = flush_buffer_since(&buffer, &mut cursor);
        assert_eq!(flushed.len(), 1);
        assert_eq!(cursor, 1);

        buffer.push(ConsoleEntry { timestamp: 2.0, level: "warn".to_string(), text: "warning".to_string() });
        buffer.push(ConsoleEntry { timestamp: 3.0, level: "error".to_string(), text: "error".to_string() });

        let flushed2 = flush_buffer_since(&buffer, &mut cursor);
        assert_eq!(flushed2.len(), 2);
        assert_eq!(flushed2[0].level, "warn");
        assert_eq!(flushed2[1].level, "error");
        assert_eq!(cursor, 3);
    }

    #[test]
    fn test_flush_buffer_since_empty() {
        let buffer: Vec<NetworkEntry> = vec![];
        let mut cursor = 0usize;
        let flushed = flush_buffer_since(&buffer, &mut cursor);
        assert!(flushed.is_empty());
        assert_eq!(cursor, 0);
    }

    #[test]
    fn test_flush_buffer_since_network() {
        let buffer = vec![
            NetworkEntry { timestamp: 1.0, method: "GET".to_string(), url: "https://example.com".to_string(), status: Some(200) },
            NetworkEntry { timestamp: 2.0, method: "POST".to_string(), url: "https://example.com/api".to_string(), status: Some(201) },
            NetworkEntry { timestamp: 3.0, method: "GET".to_string(), url: "https://example.com/page".to_string(), status: None },
        ];
        let mut cursor = 0usize;

        let flushed = flush_buffer_since(&buffer, &mut cursor);
        assert_eq!(flushed.len(), 3);
        assert_eq!(cursor, 3);
        assert_eq!(flushed[0].method, "GET");
        assert_eq!(flushed[2].status, None);
    }

    #[test]
    fn test_flush_buffer_since_mutation_summary() {
        let mut buffer = vec![
            MutationSummaryEntry { timestamp: 1.0, summary: "DOM changed".to_string() },
        ];
        let mut cursor = 0usize;

        let flushed = flush_buffer_since(&buffer, &mut cursor);
        assert_eq!(flushed.len(), 1);
        assert_eq!(flushed[0].summary, "DOM changed");

        buffer.push(MutationSummaryEntry { timestamp: 2.0, summary: "Attribute modified".to_string() });
        let flushed2 = flush_buffer_since(&buffer, &mut cursor);
        assert_eq!(flushed2.len(), 1);
        assert_eq!(flushed2[0].summary, "Attribute modified");
    }

    #[test]
    fn test_recorder_event_serde_roundtrip() {
        let event = RecorderEvent {
            timestamp: 1234.5,
            event_type: "click".to_string(),
            details: serde_json::json!({"x": 100, "y": 200}),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["event_type"], "click");
        assert_eq!(json["timestamp"], 1234.5);

        let roundtrip: RecorderEvent = serde_json::from_value(json).unwrap();
        assert_eq!(roundtrip.event_type, "click");
        assert_eq!(roundtrip.timestamp, 1234.5);
    }

    #[test]
    fn test_console_entry_serde_roundtrip() {
        let entry = ConsoleEntry {
            timestamp: 100.0,
            level: "error".to_string(),
            text: "Uncaught TypeError".to_string(),
        };
        let json = serde_json::to_value(&entry).unwrap();
        let roundtrip: ConsoleEntry = serde_json::from_value(json).unwrap();
        assert_eq!(roundtrip.level, "error");
        assert_eq!(roundtrip.text, "Uncaught TypeError");
    }

    #[test]
    fn test_network_entry_serde_roundtrip() {
        let entry = NetworkEntry {
            timestamp: 200.0,
            method: "POST".to_string(),
            url: "https://api.example.com/data".to_string(),
            status: Some(404),
        };
        let json = serde_json::to_value(&entry).unwrap();
        let roundtrip: NetworkEntry = serde_json::from_value(json).unwrap();
        assert_eq!(roundtrip.method, "POST");
        assert_eq!(roundtrip.status, Some(404));
    }

    #[test]
    fn test_network_entry_serde_no_status() {
        let entry = NetworkEntry {
            timestamp: 300.0,
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            status: None,
        };
        let json = serde_json::to_value(&entry).unwrap();
        assert!(json["status"].is_null());
        let roundtrip: NetworkEntry = serde_json::from_value(json).unwrap();
        assert!(roundtrip.status.is_none());
    }

    #[test]
    fn test_mutation_summary_entry_serde_roundtrip() {
        let entry = MutationSummaryEntry {
            timestamp: 999.0,
            summary: "childList changed on #app".to_string(),
        };
        let json = serde_json::to_value(&entry).unwrap();
        let roundtrip: MutationSummaryEntry = serde_json::from_value(json).unwrap();
        assert_eq!(roundtrip.summary, "childList changed on #app");
    }

    #[tokio::test]
    async fn test_register_and_get_browser_runtime() {
        let gcx = crate::global_context::tests::make_test_gcx().await;

        let result = get_or_create_browser_runtime(gcx.clone(), "nonexistent").await;
        assert!(result.is_none());
    }
}
