pub mod models;
pub mod types;

#[cfg(feature = "voice")]
pub mod transcribe;
#[cfg(feature = "voice")]
pub mod audio_decode;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tokio::sync::{RwLock as ARwLock, mpsc, oneshot};
use tracing::info;

use crate::voice::types::{TranscribeRequest, TranscribeResult};
use crate::voice::models::WhisperModel;

pub struct VoiceService {
    #[cfg(feature = "voice")]
    ctx: ARwLock<Option<whisper_rs::WhisperContext>>,
    model_name: ARwLock<String>,
    is_downloading: AtomicBool,
    download_progress: AtomicU8,
    queue_tx: mpsc::Sender<QueuedTranscription>,
}

struct QueuedTranscription {
    request: TranscribeRequest,
    response_tx: oneshot::Sender<Result<TranscribeResult, String>>,
}

impl VoiceService {
    pub fn new() -> Arc<Self> {
        let (queue_tx, queue_rx) = mpsc::channel::<QueuedTranscription>(100);

        let service = Arc::new(Self {
            #[cfg(feature = "voice")]
            ctx: ARwLock::new(None),
            model_name: ARwLock::new("base.en".to_string()),
            is_downloading: AtomicBool::new(false),
            download_progress: AtomicU8::new(0),
            queue_tx,
        });

        let service_clone = service.clone();
        tokio::spawn(async move {
            service_clone.process_queue(queue_rx).await;
        });

        service
    }

    async fn process_queue(self: Arc<Self>, mut rx: mpsc::Receiver<QueuedTranscription>) {
        while let Some(item) = rx.recv().await {
            let result = self.do_transcribe(item.request).await;
            let _ = item.response_tx.send(result);
        }
    }

    pub async fn transcribe(&self, request: TranscribeRequest) -> Result<TranscribeResult, String> {
        let (response_tx, response_rx) = oneshot::channel();

        self.queue_tx
            .send(QueuedTranscription { request, response_tx })
            .await
            .map_err(|_| "Voice service queue full".to_string())?;

        response_rx.await.map_err(|_| "Transcription cancelled".to_string())?
    }

    #[cfg(feature = "voice")]
    async fn do_transcribe(&self, request: TranscribeRequest) -> Result<TranscribeResult, String> {
        let mut ctx_guard = self.ctx.write().await;

        if ctx_guard.is_none() {
            let model_name = self.model_name.read().await.clone();
            let whisper_model = WhisperModel::from_name(&model_name)?;

            if let Some(path) = models::model_exists(whisper_model) {
                info!("Loading model from {:?}", path);
                let ctx = transcribe::load_context(&path)?;
                *ctx_guard = Some(ctx);
            } else {
                drop(ctx_guard);
                self.download_model(&model_name).await?;
                ctx_guard = self.ctx.write().await;
            }
        }

        let ctx = ctx_guard.as_ref().ok_or("Model not loaded")?;
        transcribe::transcribe(ctx, &request)
    }

    #[cfg(not(feature = "voice"))]
    async fn do_transcribe(&self, _request: TranscribeRequest) -> Result<TranscribeResult, String> {
        Err("Voice feature not enabled. Rebuild with --features voice".to_string())
    }

    #[cfg(feature = "voice")]
    pub async fn download_model(&self, model_name: &str) -> Result<(), String> {
        if self.is_downloading.load(Ordering::SeqCst) {
            return Err("Already downloading".to_string());
        }

        self.is_downloading.store(true, Ordering::SeqCst);
        self.download_progress.store(0, Ordering::SeqCst);

        let whisper_model = WhisperModel::from_name(model_name)?;

        let progress_ref = &self.download_progress;
        let result = models::download_model(whisper_model, |p| {
            progress_ref.store(p, Ordering::SeqCst);
        }).await;

        self.is_downloading.store(false, Ordering::SeqCst);

        match result {
            Ok(path) => {
                info!("Model downloaded to {:?}", path);
                let ctx = transcribe::load_context(&path)?;
                *self.ctx.write().await = Some(ctx);
                *self.model_name.write().await = model_name.to_string();
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    #[cfg(not(feature = "voice"))]
    pub async fn download_model(&self, _model_name: &str) -> Result<(), String> {
        Err("Voice feature not enabled".to_string())
    }

    pub fn is_downloading(&self) -> bool {
        self.is_downloading.load(Ordering::SeqCst)
    }

    pub fn download_progress(&self) -> u8 {
        self.download_progress.load(Ordering::SeqCst)
    }

    pub async fn model_name(&self) -> String {
        self.model_name.read().await.clone()
    }

    #[cfg(feature = "voice")]
    pub async fn is_model_loaded(&self) -> bool {
        self.ctx.read().await.is_some()
    }

    #[cfg(not(feature = "voice"))]
    pub async fn is_model_loaded(&self) -> bool {
        false
    }

    pub fn is_enabled() -> bool {
        cfg!(feature = "voice")
    }
}

pub type SharedVoiceService = Arc<VoiceService>;
