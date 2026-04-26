import React, { useCallback, useEffect, useState } from "react";
import { Button } from "@radix-ui/themes";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { push } from "../Pages/pagesSlice";
import {
  selectNowPlaying,
  selectBuddyDiagnostics,
  selectBuddySnapshot,
  selectIsBuddyEnabled,
} from "./buddySlice";
import { PALETTES, SIGNALS } from "./constants";
import styles from "./BuddyChatCompanion.module.css";

export const BuddyChatCompanion: React.FC = () => {
  const dispatch = useAppDispatch();
  const nowPlaying = useAppSelector(selectNowPlaying);
  const diagnostics = useAppSelector(selectBuddyDiagnostics);
  const snapshot = useAppSelector(selectBuddySnapshot);
  const enabled = useAppSelector(selectIsBuddyEnabled);
  const [dismissed, setDismissed] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  useEffect(() => {
    if (nowPlaying && SIGNALS[nowPlaying.signal_type]?.isError) {
      setMsg(nowPlaying.title);
      setDismissed(false);
    }
  }, [nowPlaying?.id]);

  useEffect(() => {
    const diag = diagnostics[0];
    if (diag) {
      setMsg(diag.error_message.slice(0, 120));
      setDismissed(false);
    }
  }, [diagnostics.length]);

  useEffect(() => {
    if (!msg || dismissed) return;
    const t = setTimeout(() => setDismissed(true), 30000);
    return () => clearTimeout(t);
  }, [msg, dismissed]);

  const handleOpen = useCallback(() => {
    dispatch(push({ name: "buddy" }));
  }, [dispatch]);

  const paletteIdx = snapshot?.state.identity.palette_index ?? 0;
  const color = PALETTES[paletteIdx]?.body ?? PALETTES[0].body;

  if (!enabled || !msg || dismissed) return null;

  return (
    <div className={styles.companion}>
      <div className={styles.face} style={{ background: color }}>
        🤔
      </div>
      <div className={styles.text}>{msg}</div>
      <div className={styles.actions}>
        <Button size="1" variant="ghost" onClick={handleOpen}>
          Ask Buddy
        </Button>
        <Button
          size="1"
          variant="ghost"
          color="gray"
          onClick={() => setDismissed(true)}
        >
          ×
        </Button>
      </div>
    </div>
  );
};
