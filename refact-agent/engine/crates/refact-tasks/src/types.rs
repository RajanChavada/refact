use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMeta {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub cards_total: usize,
    #[serde(default)]
    pub cards_done: usize,
    #[serde(default)]
    pub cards_failed: usize,
    #[serde(default)]
    pub agents_active: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_agent_model: Option<String>,
    #[serde(default)]
    pub is_name_generated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_agents_summary_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planner_session_state: Option<String>,
}

fn default_schema_version() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Planning,
    Active,
    Paused,
    Completed,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBoard {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub rev: u64,
    #[serde(default = "default_columns")]
    pub columns: Vec<BoardColumn>,
    #[serde(default)]
    pub cards: Vec<BoardCard>,
}

fn default_columns() -> Vec<BoardColumn> {
    vec![
        BoardColumn {
            id: "planned".into(),
            title: "Planned".into(),
        },
        BoardColumn {
            id: "doing".into(),
            title: "Doing".into(),
        },
        BoardColumn {
            id: "done".into(),
            title: "Done".into(),
        },
        BoardColumn {
            id: "failed".into(),
            title: "Failed".into(),
        },
    ]
}

impl Default for TaskBoard {
    fn default() -> Self {
        Self {
            schema_version: 1,
            rev: 0,
            columns: default_columns(),
            cards: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardColumn {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardCard {
    pub id: String,
    pub title: String,
    pub column: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub instructions: String,
    pub assignee: Option<String>,
    pub agent_chat_id: Option<String>,
    #[serde(default)]
    pub status_updates: Vec<StatusUpdate>,
    pub final_report: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_at: Option<String>,
    pub completed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_worktree: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_worktree_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_files: Vec<String>,
}

fn default_priority() -> String {
    "P1".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub timestamp: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyCardsResult {
    pub ready: Vec<String>,
    pub blocked: Vec<String>,
    pub in_progress: Vec<String>,
    pub completed: Vec<String>,
    pub failed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryInfo {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_state: Option<String>,
}

impl TaskBoard {
    pub fn get_ready_cards(&self) -> ReadyCardsResult {
        let mut ready = vec![];
        let mut blocked = vec![];
        let mut in_progress = vec![];
        let mut completed = vec![];
        let mut failed = vec![];

        let done_cards: std::collections::HashSet<_> = self
            .cards
            .iter()
            .filter(|c| c.column == "done")
            .map(|c| c.id.as_str())
            .collect();

        for card in &self.cards {
            match card.column.as_str() {
                "done" => completed.push(card.id.clone()),
                "failed" => failed.push(card.id.clone()),
                "doing" => in_progress.push(card.id.clone()),
                "planned" => {
                    let deps_satisfied = card
                        .depends_on
                        .iter()
                        .all(|dep| done_cards.contains(dep.as_str()));
                    if deps_satisfied {
                        ready.push(card.id.clone());
                    } else {
                        blocked.push(card.id.clone());
                    }
                }
                _ => {}
            }
        }

        ReadyCardsResult {
            ready,
            blocked,
            in_progress,
            completed,
            failed,
        }
    }

    pub fn get_card(&self, card_id: &str) -> Option<&BoardCard> {
        self.cards.iter().find(|c| c.id == card_id)
    }

    pub fn get_card_mut(&mut self, card_id: &str) -> Option<&mut BoardCard> {
        self.cards.iter_mut().find(|c| c.id == card_id)
    }

    pub fn get_dependency_reports(&self, card_id: &str) -> Vec<(String, String)> {
        let card = match self.get_card(card_id) {
            Some(c) => c,
            None => return vec![],
        };

        card.depends_on
            .iter()
            .filter_map(|dep_id| {
                self.get_card(dep_id).and_then(|dep_card| {
                    dep_card
                        .final_report
                        .as_ref()
                        .map(|report| (dep_card.title.clone(), report.clone()))
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, title: &str, column: &str, depends_on: Vec<&str>) -> BoardCard {
        BoardCard {
            id: id.into(),
            title: title.into(),
            column: column.into(),
            priority: default_priority(),
            depends_on: depends_on.into_iter().map(String::from).collect(),
            instructions: String::new(),
            assignee: None,
            agent_chat_id: None,
            status_updates: vec![],
            final_report: None,
            created_at: "2026-05-16T00:00:00Z".into(),
            started_at: None,
            last_heartbeat_at: None,
            completed_at: None,
            agent_branch: None,
            agent_worktree: None,
            agent_worktree_name: None,
            target_files: vec![],
        }
    }

    #[test]
    fn default_board_has_schema_and_columns() {
        let board = TaskBoard::default();

        assert_eq!(board.schema_version, 1);
        assert_eq!(board.rev, 0);
        assert!(board.cards.is_empty());
        assert_eq!(
            board
                .columns
                .iter()
                .map(|column| (column.id.as_str(), column.title.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("planned", "Planned"),
                ("doing", "Doing"),
                ("done", "Done"),
                ("failed", "Failed")
            ]
        );
    }

    #[test]
    fn serde_defaults_preserve_schema_values() {
        let meta: TaskMeta = serde_json::from_str(
            r#"{
                "id": "task-1",
                "name": "Task One",
                "status": "active",
                "created_at": "created",
                "updated_at": "updated"
            }"#,
        )
        .unwrap();
        let board: TaskBoard = serde_json::from_str(r#"{"cards": []}"#).unwrap();

        assert_eq!(meta.schema_version, 1);
        assert_eq!(meta.cards_total, 0);
        assert_eq!(meta.cards_done, 0);
        assert_eq!(meta.cards_failed, 0);
        assert_eq!(meta.agents_active, 0);
        assert!(!meta.is_name_generated);
        assert_eq!(board.schema_version, 1);
        assert_eq!(board.rev, 0);
        assert_eq!(board.columns.len(), 4);
    }

    #[test]
    fn ready_cards_separate_ready_blocked_and_terminal_columns() {
        let board = TaskBoard {
            cards: vec![
                card("dep-done", "Dependency done", "done", vec![]),
                card("dep-failed", "Dependency failed", "failed", vec![]),
                card("ready", "Ready", "planned", vec!["dep-done"]),
                card("blocked", "Blocked", "planned", vec!["dep-failed"]),
                card("blocked-missing", "Blocked missing", "planned", vec!["missing"]),
                card("in-progress", "In progress", "doing", vec![]),
            ],
            ..TaskBoard::default()
        };

        let result = board.get_ready_cards();

        assert_eq!(result.ready, vec!["ready"]);
        assert_eq!(result.blocked, vec!["blocked", "blocked-missing"]);
        assert_eq!(result.in_progress, vec!["in-progress"]);
        assert_eq!(result.completed, vec!["dep-done"]);
        assert_eq!(result.failed, vec!["dep-failed"]);
    }

    #[test]
    fn dependency_reports_include_only_dependencies_with_reports() {
        let mut reported = card("dep-reported", "Reported dependency", "done", vec![]);
        reported.final_report = Some("finished cleanly".into());
        let unreported = card("dep-unreported", "Unreported dependency", "done", vec![]);
        let consumer = card(
            "consumer",
            "Consumer",
            "planned",
            vec!["dep-reported", "dep-unreported", "missing"],
        );
        let board = TaskBoard {
            cards: vec![reported, unreported, consumer],
            ..TaskBoard::default()
        };

        assert_eq!(
            board.get_dependency_reports("consumer"),
            vec![("Reported dependency".into(), "finished cleanly".into())]
        );
        assert!(board.get_dependency_reports("missing").is_empty());
    }
}
