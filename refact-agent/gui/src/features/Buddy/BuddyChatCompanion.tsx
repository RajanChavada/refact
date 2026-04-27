import React, { useEffect, useMemo, useRef, useState } from "react";
import { useAppDispatch, useAppSelector } from "../../hooks";
import {
  selectNowPlaying,
  selectBuddyDiagnostics,
  selectIsBuddyEnabled,
  selectRuntimeQueue,
  selectBuddySuggestions,
  setActiveSpeech,
  clearActiveSpeech,
} from "./buddySlice";

interface Props {
  chatId: string;
}

// Logic-only component: detects chat errors and pushes them into activeSpeech
// so the existing BuddySpeechCloud (in BuddyPanel) renders the cloud above Buddy.
export const BuddyChatCompanion: React.FC<Props> = ({ chatId }) => {
  const dispatch = useAppDispatch();
  const enabled = useAppSelector(selectIsBuddyEnabled);
  const runtimeQueue = useAppSelector(selectRuntimeQueue);
  const nowPlaying = useAppSelector(selectNowPlaying);
  const diagnostics = useAppSelector(selectBuddyDiagnostics);
  const suggestions = useAppSelector(selectBuddySuggestions);

  const [dismissed, setDismissed] = useState(false);
  const prevChatIdRef = useRef(chatId);

  useEffect(() => {
    if (prevChatIdRef.current !== chatId) {
      prevChatIdRef.current = chatId;
      setDismissed(false);
    }
  }, [chatId]);

  const chatError = useMemo(() => {
    if (nowPlaying?.chat_id === chatId && nowPlaying?.status === "failed") {
      return nowPlaying;
    }
    return (
      runtimeQueue.find((e) => e.chat_id === chatId && e.status === "failed") ??
      null
    );
  }, [runtimeQueue, nowPlaying, chatId]);

  const chatDiagnostic = useMemo(
    () => diagnostics.find((d) => d.chat_id === chatId),
    [diagnostics, chatId],
  );

  const errorSuggestion = useMemo(
    () =>
      suggestions.find(
        (s) => !s.dismissed && s.suggestion_type === "error_pattern",
      ),
    [suggestions],
  );

  const isErrorSuggestionMode =
    !chatError && !chatDiagnostic && !!errorSuggestion;

  const message =
    chatError?.title ??
    chatDiagnostic?.error_message?.slice(0, 120) ??
    (errorSuggestion ? errorSuggestion.description : null);

  useEffect(() => {
    if (!enabled || !message || dismissed) {
      dispatch(clearActiveSpeech());
      return;
    }
    dispatch(
      setActiveSpeech({
        id: `chat-error-${chatId}`,
        text: message,
        mood: "concerned",
        scope: "chat",
        persistent: false,
        ttl_seconds: 15,
        created_at: new Date().toISOString(),
        controls: isErrorSuggestionMode
          ? [
              {
                id: "investigate",
                label: "Investigate →",
                action: "investigate_error",
                style: "primary",
              },
            ]
          : [
              {
                id: "ask",
                label: "Ask Buddy",
                action: "open_buddy",
                style: "ghost",
              },
            ],
      }),
    );
  }, [enabled, message, dismissed, chatId, isErrorSuggestionMode, dispatch]);

  useEffect(() => {
    if (!message || dismissed) return;
    const t = setTimeout(() => {
      setDismissed(true);
      dispatch(clearActiveSpeech());
    }, 15000);
    return () => clearTimeout(t);
  }, [message, dismissed, dispatch]);

  useEffect(() => {
    return () => {
      dispatch(clearActiveSpeech());
    };
  }, [dispatch]);

  return null;
};
