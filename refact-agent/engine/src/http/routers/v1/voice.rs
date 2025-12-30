use std::sync::Arc;
use axum::Extension;
use axum::response::Response;
use hyper::{Body, StatusCode};
use tokio::sync::RwLock as ARwLock;

use crate::custom_error::ScratchError;
use crate::global_context::GlobalContext;
use crate::voice::types::*;
use crate::voice::models::WhisperModel;

pub async fn handle_v1_voice_transcribe(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let req: TranscribeRequest = serde_json::from_slice(&body)
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    let gcx_locked = gcx.read().await;
    let voice_service = gcx_locked.voice_service.clone();
    drop(gcx_locked);

    let result = voice_service
        .transcribe(req)
        .await
        .map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let response = TranscribeResponse {
        text: result.text,
        language: result.language,
        duration_ms: result.duration_ms,
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&response).unwrap()))
        .unwrap())
}

pub async fn handle_v1_voice_download(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
    body: hyper::body::Bytes,
) -> Result<Response<Body>, ScratchError> {
    let req: DownloadModelRequest = serde_json::from_slice(&body)
        .unwrap_or(DownloadModelRequest { model: "base.en".to_string() });

    WhisperModel::from_name(&req.model)
        .map_err(|e| ScratchError::new(StatusCode::BAD_REQUEST, e))?;

    let gcx_locked = gcx.read().await;
    let voice_service = gcx_locked.voice_service.clone();
    drop(gcx_locked);

    let voice_service_clone = voice_service.clone();
    let model_name = req.model.clone();
    tokio::spawn(async move {
        let _ = voice_service_clone.download_model(&model_name).await;
    });

    let response = DownloadModelResponse {
        success: true,
        message: format!("Download started for model: {}", req.model),
    };

    Ok(Response::builder()
        .status(StatusCode::ACCEPTED)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&response).unwrap()))
        .unwrap())
}

pub async fn handle_v1_voice_status(
    Extension(gcx): Extension<Arc<ARwLock<GlobalContext>>>,
) -> Result<Response<Body>, ScratchError> {
    let gcx_locked = gcx.read().await;
    let voice_service = gcx_locked.voice_service.clone();
    drop(gcx_locked);

    let response = VoiceStatusResponse {
        enabled: crate::voice::VoiceService::is_enabled(),
        model_loaded: voice_service.is_model_loaded().await,
        model_name: voice_service.model_name().await,
        is_downloading: voice_service.is_downloading(),
        download_progress: voice_service.download_progress(),
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&response).unwrap()))
        .unwrap())
}
