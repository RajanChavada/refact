use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::sync::{Mutex as AMutex, Notify};
use tokio::time::{sleep, sleep_until, Instant as TokioInstant};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage, ContextEnum};
use crate::chat::internal_roles::{event, EventSubkind};
use crate::postprocessing::pp_command_output::OutputFilter;
use crate::tools::tools_description::{Tool, ToolDesc, ToolSource, ToolSourceType};

const MIN_DURATION_MS: u64 = 100;
const MAX_DURATION_MS: u64 = 3_600_000;
pub struct ToolSleep {
    pub config_path: String,
}

#[derive(Clone)]
struct SleepRequest {
    duration_ms: u64,
    tick_interval_ms: Option<u64>,
    description: String,
}

struct SleepOutcome {
    slept_ms: u64,
    interrupted: bool,
    ticks: Vec<ChatMessage>,
}

#[async_trait]
impl Tool for ToolSleep {
    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "sleep".to_string(),
            display_name: "Sleep".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            experimental: false,
            allow_parallel: true,
            description: "Wait for the specified duration. User-interruptible at any time. Use when you have nothing to do, when waiting for something, or when the user asks you to pause. Prefer this over Bash(sleep ...) — it doesn't hold a shell process. You can call this concurrently with other tools.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "duration_ms": {
                        "type": "integer",
                        "minimum": 100,
                        "maximum": 3600000,
                        "description": "Sleep duration in ms (max 1h)."
                    },
                    "tick_interval_ms": {
                        "type": "integer",
                        "minimum": 5000,
                        "description": "Optional. If set, inject event(tick) at each interval so you can react mid-sleep."
                    },
                    "description": {
                        "type": "string",
                        "description": "Short description (≤80 chars)."
                    }
                },
                "required": ["duration_ms", "description"]
            }),
            output_schema: None,
            annotations: None,
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let request = parse_sleep_request(args)?;
        let (abort_flag, app, chat_id) = {
            let ccx = ccx.lock().await;
            (ccx.abort_flag.clone(), ccx.app.clone(), ccx.chat_id.clone())
        };
        let abort_notify = find_abort_notify(app.clone(), chat_id.clone()).await;
        let outcome = sleep_with_ticks(
            request.duration_ms,
            request.tick_interval_ms,
            abort_flag,
            abort_notify,
            |tick| inject_tick(app.clone(), chat_id.clone(), tick),
        )
        .await;

        let body = json!({
            "slept_ms": outcome.slept_ms,
            "interrupted": outcome.interrupted,
        });
        let mut extra = serde_json::Map::new();
        extra.insert("sleep".to_string(), body.clone());
        let mut messages = vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(body.to_string()),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            tool_failed: Some(false),
            output_filter: Some(OutputFilter::no_limits()),
            extra,
            ..Default::default()
        })];
        messages.extend(outcome.ticks.into_iter().map(ContextEnum::ChatMessage));
        tracing::info!(
            slept_ms = outcome.slept_ms,
            interrupted = outcome.interrupted,
            description = %request.description,
            "sleep tool completed"
        );
        Ok((false, messages))
    }
}

fn parse_sleep_request(args: &HashMap<String, Value>) -> Result<SleepRequest, String> {
    let duration_ms = required_u64(args, "duration_ms")?;
    if !(MIN_DURATION_MS..=MAX_DURATION_MS).contains(&duration_ms) {
        return Err(format!(
            "duration_ms must be between {MIN_DURATION_MS} and {MAX_DURATION_MS}"
        ));
    }

    let tick_interval_ms = optional_u64(args, "tick_interval_ms")?;

    let description = match args.get("description") {
        Some(Value::String(description)) => description.clone(),
        Some(_) => return Err("description must be a string".to_string()),
        None => return Err("Missing required argument 'description'".to_string()),
    };
    if description.chars().count() > 80 {
        return Err("description must be at most 80 chars".to_string());
    }

    Ok(SleepRequest {
        duration_ms,
        tick_interval_ms,
        description,
    })
}

fn required_u64(args: &HashMap<String, Value>, name: &str) -> Result<u64, String> {
    args.get(name)
        .ok_or_else(|| format!("Missing required argument '{name}'"))
        .and_then(|value| {
            value
                .as_u64()
                .ok_or_else(|| format!("{name} must be an integer"))
        })
}

fn optional_u64(args: &HashMap<String, Value>, name: &str) -> Result<Option<u64>, String> {
    args.get(name)
        .map(|value| {
            value
                .as_u64()
                .ok_or_else(|| format!("{name} must be an integer"))
        })
        .transpose()
}

async fn sleep_with_ticks<F, Fut>(
    duration_ms: u64,
    tick_interval_ms: Option<u64>,
    abort_flag: Arc<AtomicBool>,
    abort_notify: Option<Arc<Notify>>,
    mut on_tick: F,
) -> SleepOutcome
where
    F: FnMut(ChatMessage) -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let started = Instant::now();
    let end = TokioInstant::now() + Duration::from_millis(duration_ms);
    let mut ticks = Vec::new();

    loop {
        if abort_flag.load(Ordering::Relaxed) {
            return SleepOutcome {
                slept_ms: elapsed_ms(started),
                interrupted: true,
                ticks,
            };
        }

        let now = TokioInstant::now();
        if now >= end {
            return SleepOutcome {
                slept_ms: elapsed_ms(started),
                interrupted: false,
                ticks,
            };
        }

        let tick_sleep = tick_interval_ms
            .map(Duration::from_millis)
            .filter(|interval| *interval < end.saturating_duration_since(now))
            .map(sleep);
        tokio::pin!(tick_sleep);

        tokio::select! {
            _ = sleep_until(end) => {
                return SleepOutcome {
                    slept_ms: elapsed_ms(started),
                    interrupted: false,
                    ticks,
                };
            }
            _ = wait_for_abort(abort_flag.clone(), abort_notify.clone()) => {
                return SleepOutcome {
                    slept_ms: elapsed_ms(started),
                    interrupted: true,
                    ticks,
                };
            }
            _ = async {
                if let Some(tick_sleep) = tick_sleep.as_mut().as_pin_mut() {
                    tick_sleep.await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                let elapsed_ms = elapsed_ms(started).min(duration_ms);
                let remaining_ms = duration_ms.saturating_sub(elapsed_ms);
                let tick = tick_event(elapsed_ms, remaining_ms);
                if !on_tick(tick.clone()).await {
                    ticks.push(tick);
                }
            }
        }
    }
}

async fn wait_for_abort(abort_flag: Arc<AtomicBool>, abort_notify: Option<Arc<Notify>>) {
    loop {
        if abort_flag.load(Ordering::Relaxed) {
            return;
        }
        if let Some(abort_notify) = &abort_notify {
            abort_notify.notified().await;
        } else {
            sleep(Duration::from_millis(10)).await;
        }
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn tick_event(elapsed_ms: u64, remaining_ms: u64) -> ChatMessage {
    event(
        EventSubkind::Tick,
        "tool.sleep",
        json!({
            "elapsed_ms": elapsed_ms,
            "remaining_ms": remaining_ms,
        }),
        "tick",
    )
}

async fn find_abort_notify(
    app: crate::app_state::AppState,
    chat_id: String,
) -> Option<Arc<Notify>> {
    let session = {
        let sessions = app.chat.sessions.read().await;
        sessions.get(&chat_id).cloned()
    }?;
    let abort_notify = {
        let session = session.lock().await;
        session.abort_notify.clone()
    };
    Some(abort_notify)
}

async fn inject_tick(app: crate::app_state::AppState, chat_id: String, tick: ChatMessage) -> bool {
    let session = {
        let sessions = app.chat.sessions.read().await;
        sessions.get(&chat_id).cloned()
    };
    if let Some(session) = session {
        session.lock().await.add_message(tick);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[tokio::test]
    async fn short_sleep_returns_correct_slept_ms() {
        let outcome = sleep_with_ticks(
            120,
            None,
            Arc::new(AtomicBool::new(false)),
            None,
            |_| async { false },
        )
        .await;

        assert!(!outcome.interrupted);
        assert!(
            (70..=170).contains(&outcome.slept_ms),
            "slept_ms was {}",
            outcome.slept_ms
        );
        assert!(outcome.ticks.is_empty());
    }

    #[tokio::test]
    async fn abort_midway_returns_interrupted() {
        let abort_flag = Arc::new(AtomicBool::new(false));
        let run = tokio::spawn({
            let abort_flag = abort_flag.clone();
            async move { sleep_with_ticks(2_000, None, abort_flag, None, |_| async { false }).await }
        });

        sleep(Duration::from_millis(120)).await;
        abort_flag.store(true, Ordering::Relaxed);
        let outcome = run.await.unwrap();

        assert!(outcome.interrupted);
        assert!(outcome.slept_ms < 500, "slept_ms was {}", outcome.slept_ms);
    }

    #[tokio::test]
    async fn tick_interval_injects_n_events() {
        let outcome = sleep_with_ticks(
            600,
            Some(200),
            Arc::new(AtomicBool::new(false)),
            None,
            |_| async { false },
        )
        .await;

        assert!(!outcome.interrupted);
        assert!(
            (2..=3).contains(&outcome.ticks.len()),
            "tick count was {}",
            outcome.ticks.len()
        );
        assert!(outcome.ticks.iter().all(|message| message.role == "event"));
        assert_eq!(outcome.ticks[0].extra["event"]["subkind"], json!("tick"));
    }
}
