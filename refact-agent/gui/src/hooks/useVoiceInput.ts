import { useState, useCallback, useEffect } from "react";
import { useVoiceRecording } from "./useVoiceRecording";
import {
  transcribeAudio,
  getVoiceStatus,
  downloadVoiceModel,
  VoiceStatusResponse,
} from "../services/refact/voice";

export interface UseVoiceInputResult {
  isRecording: boolean;
  isTranscribing: boolean;
  isDownloading: boolean;
  downloadProgress: number;
  error: string | null;
  voiceEnabled: boolean;
  modelLoaded: boolean;
  toggleRecording: () => Promise<void>;
}

export function useVoiceInput(
  onTranscript: (text: string) => void,
): UseVoiceInputResult {
  const {
    isRecording,
    error: recordingError,
    toggleRecording: toggle,
  } = useVoiceRecording();
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<VoiceStatusResponse | null>(null);

  useEffect(() => {
    if (recordingError) {
      setError(recordingError);
    }
  }, [recordingError]);

  useEffect(() => {
    getVoiceStatus()
      .then(setStatus)
      .catch(() => setStatus(null));
  }, []);

  useEffect(() => {
    if (!status?.is_downloading) return;

    const interval = setInterval(() => {
      getVoiceStatus()
        .then(setStatus)
        .catch(() => { /* ignore polling errors */ });
    }, 1000);

    return () => clearInterval(interval);
  }, [status?.is_downloading]);

  const toggleRecording = useCallback(async () => {
    setError(null);

    if (isRecording) {
      const blob = await toggle();
      if (!blob || blob.size < 1000) {
        setError("Recording too short. Please hold the mic button for 2-3 seconds while speaking.");
        return;
      }

      setIsTranscribing(true);

      try {
        const reader = new FileReader();
        const base64Promise = new Promise<string>((resolve, reject) => {
          reader.onloadend = () => {
            const result = reader.result as string;
            resolve(result);
          };
          reader.onerror = reject;
        });
        reader.readAsDataURL(blob);

        const audioData = await base64Promise;
        const response = await transcribeAudio({
          audio_data: audioData,
          mime_type: blob.type,
        });

        if (response.text.trim()) {
          onTranscript(response.text.trim());
        }
      } catch (err) {
        const message =
          err instanceof Error ? err.message : "Transcription failed";

        if (message.includes("Model not downloaded")) {
          downloadVoiceModel().catch(() => { /* ignore download errors */ });
          const newStatus = await getVoiceStatus().catch(() => null);
          if (newStatus) setStatus(newStatus);
        }

        setError(message);
      } finally {
        setIsTranscribing(false);
      }
    } else {
      await toggle();
    }
  }, [isRecording, toggle, onTranscript]);

  return {
    isRecording,
    isTranscribing,
    isDownloading: status?.is_downloading ?? false,
    downloadProgress: status?.download_progress ?? 0,
    error,
    voiceEnabled: status?.enabled ?? false,
    modelLoaded: status?.model_loaded ?? false,
    toggleRecording,
  };
}
