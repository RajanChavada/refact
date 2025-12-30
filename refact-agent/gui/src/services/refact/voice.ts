const VOICE_API_BASE = "/v1/voice";

export interface TranscribeRequest {
  audio_data: string;
  mime_type?: string;
  language?: string;
}

export interface TranscribeResponse {
  text: string;
  language: string;
  duration_ms: number;
}

export interface VoiceStatusResponse {
  enabled: boolean;
  model_loaded: boolean;
  model_name: string;
  is_downloading: boolean;
  download_progress: number;
}

export interface DownloadModelRequest {
  model?: string;
}

export interface DownloadModelResponse {
  success: boolean;
  message: string;
}

export async function transcribeAudio(
  request: TranscribeRequest,
): Promise<TranscribeResponse> {
  const response = await fetch(`${VOICE_API_BASE}/transcribe`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(error || "Transcription failed");
  }

  return response.json() as Promise<TranscribeResponse>;
}

export async function getVoiceStatus(): Promise<VoiceStatusResponse> {
  const response = await fetch(`${VOICE_API_BASE}/status`);
  if (!response.ok) {
    throw new Error("Failed to get voice status");
  }
  return response.json() as Promise<VoiceStatusResponse>;
}

export async function downloadVoiceModel(
  model?: string,
): Promise<DownloadModelResponse> {
  const response = await fetch(`${VOICE_API_BASE}/download`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ model: model ?? "base.en" }),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(error || "Download failed");
  }

  return response.json() as Promise<DownloadModelResponse>;
}
