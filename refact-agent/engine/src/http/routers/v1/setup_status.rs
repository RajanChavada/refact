use axum::Extension;
use axum::response::Result;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::ScratchError;
use crate::files_correction::get_project_dirs;
use crate::global_context::GlobalContext;

#[derive(Serialize)]
pub struct SetupStatusResponse {
    pub configured: bool,
    pub reasons: Vec<String>,
    pub paths: SetupStatusPaths,
}

#[derive(Serialize)]
pub struct SetupStatusPaths {
    pub project_root: Option<String>,
    pub agents_md: Option<String>,
    pub project_summary: Option<String>,
    pub refact_dir: Option<String>,
}

fn first_project_root(project_dirs: &[PathBuf]) -> Option<PathBuf> {
    project_dirs.first().cloned()
}

pub async fn handle_v1_setup_status(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
) -> Result<axum::Json<SetupStatusResponse>, ScratchError> {
    let project_dirs = get_project_dirs(gcx).await;
    let project_root = first_project_root(&project_dirs);

    if project_root.is_none() {
        return Ok(axum::Json(SetupStatusResponse {
            configured: true,
            reasons: vec![],
            paths: SetupStatusPaths {
                project_root: None,
                agents_md: None,
                project_summary: None,
                refact_dir: None,
            },
        }));
    }

    let (agents_md, project_summary, refact_dir) = if let Some(root) = &project_root {
        (
            root.join("AGENTS.md"),
            root.join(".refact").join("project_summary.yaml"),
            root.join(".refact"),
        )
    } else {
        (PathBuf::new(), PathBuf::new(), PathBuf::new())
    };

    let agents_exists = !agents_md.as_os_str().is_empty() && agents_md.exists();
    let summary_exists = !project_summary.as_os_str().is_empty() && project_summary.exists();
    let refact_exists = !refact_dir.as_os_str().is_empty() && refact_dir.exists();

    let mut reasons = Vec::new();
    if !agents_exists {
        reasons.push("missing_agents_md".to_string());
    }
    if !summary_exists {
        reasons.push("missing_project_summary".to_string());
    }
    if !refact_exists {
        reasons.push("missing_refact_dir".to_string());
    }

    let agents_md_path = if agents_md.as_os_str().is_empty() {
        None
    } else {
        Some(agents_md.to_string_lossy().to_string())
    };
    let project_summary_path = if project_summary.as_os_str().is_empty() {
        None
    } else {
        Some(project_summary.to_string_lossy().to_string())
    };
    let refact_dir_path = if refact_dir.as_os_str().is_empty() {
        None
    } else {
        Some(refact_dir.to_string_lossy().to_string())
    };

    Ok(axum::Json(SetupStatusResponse {
        configured: reasons.is_empty(),
        reasons,
        paths: SetupStatusPaths {
            project_root: project_root.map(|p| p.to_string_lossy().to_string()),
            agents_md: agents_md_path,
            project_summary: project_summary_path,
            refact_dir: refact_dir_path,
        },
    }))
}
