import React, { useCallback } from "react";
import { Button } from "@radix-ui/themes";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { push } from "../Pages/pagesSlice";
import { clearActiveSpeech, selectActiveSpeech } from "./buddySlice";
import {
  openBuddyChat,
  newBuddyChatAction,
  openChatInModeAndStart,
} from "../Chat/Thread";
import { useCreateBuddyConversationMutation } from "../../services/refact/buddy";
import type { BuddyControl } from "./types";
import styles from "./BuddySpeechCloud.module.css";

interface Props {
  variant?: "block" | "overlay";
}

export const BuddySpeechCloud: React.FC<Props> = ({ variant = "block" }) => {
  const dispatch = useAppDispatch();
  const speech = useAppSelector(selectActiveSpeech);
  const [createConversation] = useCreateBuddyConversationMutation();

  const handleControl = useCallback(
    async (ctrl: BuddyControl) => {
      switch (ctrl.action) {
        case "dismiss":
          dispatch(clearActiveSpeech());
          break;
        case "open_setup":
          void dispatch(openChatInModeAndStart({ mode: "setup" }));
          dispatch(clearActiveSpeech());
          break;
        case "open_stats":
          dispatch(push({ name: "stats dashboard" }));
          dispatch(clearActiveSpeech());
          break;
        case "open_buddy":
          dispatch(push({ name: "buddy" }));
          dispatch(clearActiveSpeech());
          break;
        case "investigate_error": {
          dispatch(clearActiveSpeech());
          const result = await createConversation(undefined);
          if ("data" in result && result.data) {
            const meta = result.data;
            dispatch(newBuddyChatAction({ chat_id: meta.chat_id }));
            dispatch(
              openBuddyChat({ chat_id: meta.chat_id, title: meta.title }),
            );
            dispatch(push({ name: "chat" }));
          }
          break;
        }
        default:
          dispatch(clearActiveSpeech());
      }
    },
    [dispatch, createConversation],
  );

  if (!speech) return null;

  const isOverlay = variant === "overlay";

  return (
    <div className={isOverlay ? styles.cloudOverlay : styles.cloud}>
      <p className={isOverlay ? styles.overlayText : styles.text}>
        {speech.text}
      </p>
      <div className={styles.controls}>
        {speech.controls.map((ctrl) => (
          <Button
            key={ctrl.id}
            size="1"
            variant={ctrl.style === "primary" ? "solid" : "soft"}
            onClick={() => void handleControl(ctrl)}
          >
            {ctrl.label}
          </Button>
        ))}
        <Button
          size="1"
          variant="ghost"
          color="gray"
          onClick={() => dispatch(clearActiveSpeech())}
        >
          ✕
        </Button>
      </div>
      <div className={isOverlay ? styles.overlayTail : styles.tail} />
    </div>
  );
};
