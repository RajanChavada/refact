import React, { useCallback, useState } from "react";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { push } from "../Pages/pagesSlice";
import { BuddyCanvas } from "./BuddyCanvas";
import { useBuddyState } from "./hooks/useBuddyState";
import { useBuddyOpportunities } from "./hooks/useBuddyOpportunities";
import {
  selectBuddySnapshot,
  selectIsBuddyEnabled,
  selectNowPlaying,
  selectActiveSpeech,
  selectBuddyDiagnostics,
} from "./buddySlice";
import { executeBuddyAction } from "./executeBuddyAction";
import type { BuddyControl } from "./types";
import { PALETTES, SIGNALS } from "./constants";
import { useExecuteBuddyAction } from "./hooks/useExecuteBuddyAction";
import {
  getOpportunityActionFromControl,
  opportunityActionControls,
  opportunitySpeechText,
} from "./buddyOpportunityActions";
import styles from "./BuddyPanel.module.css";

export const BuddyPanel: React.FC = () => {
  const dispatch = useAppDispatch();
  const snapshot = useAppSelector(selectBuddySnapshot);
  const enabled = useAppSelector(selectIsBuddyEnabled);
  const nowPlaying = useAppSelector(selectNowPlaying);
  const activeSpeech = useAppSelector(selectActiveSpeech);
  const diagnostics = useAppSelector(selectBuddyDiagnostics);
  const { unread } = useBuddyOpportunities();
  const [showTop, setShowTop] = useState(false);
  const executeOpportunityAction = useExecuteBuddyAction();

  const buddy = useBuddyState();
  const { state } = buddy;

  const activeDiagnostic = activeSpeech?.chat_id
    ? diagnostics.find((diag) => diag.chat_id === activeSpeech.chat_id)
    : undefined;

  const paletteIndex =
    snapshot?.state.identity.palette_index ?? state.paletteIndex;
  const palette = PALETTES[paletteIndex] ?? PALETTES[0];

  const topOpportunity = showTop ? unread[0] ?? null : null;
  const speechText = topOpportunity
    ? opportunitySpeechText(topOpportunity)
    : activeSpeech
      ? activeSpeech.text
      : nowPlaying?.speech_text ?? nowPlaying?.title ?? null;
  const speechControls = topOpportunity
    ? opportunityActionControls(topOpportunity)
    : activeSpeech
      ? activeSpeech.controls
      : undefined;
  const speechHandler = topOpportunity
    ? async (ctrl: BuddyControl) => {
        const action = getOpportunityActionFromControl(ctrl, topOpportunity);
        if (!action) return;
        await executeOpportunityAction(action, topOpportunity);
        setShowTop(false);
      }
    : activeSpeech
      ? async (ctrl: BuddyControl) => {
          await executeBuddyAction(ctrl, dispatch, {
            triggerText: activeSpeech.text,
            triggerSource: "runtime",
            sourceChatId: activeSpeech.chat_id,
            diagnostic: activeDiagnostic,
          });
        }
      : undefined;

  const handleOpen = useCallback(() => {
    dispatch(push({ name: "buddy" }));
  }, [dispatch]);

  if (snapshot === null) return null;
  if (!enabled) return null;

  const badgeCount = unread.length;
  const badgeLabel = badgeCount > 9 ? "9+" : String(badgeCount);

  return (
    <div
      className={styles.block}
      onClick={handleOpen}
      style={{ cursor: "pointer" }}
    >
      {badgeCount > 0 && (
        <div
          style={{
            position: "relative",
            display: "flex",
            justifyContent: "flex-end",
            paddingRight: "var(--space-2)",
          }}
        >
          <button
            type="button"
            data-testid="buddy-unread-badge"
            aria-label={`${badgeCount} unread opportunities`}
            style={{
              background: "var(--accent-9)",
              color: "var(--accent-contrast)",
              border: "none",
              borderRadius: "9999px",
              padding: "1px 5px",
              fontSize: "10px",
              fontWeight: 700,
              cursor: "pointer",
              lineHeight: 1.4,
            }}
            onClick={(e) => {
              e.stopPropagation();
              setShowTop((v) => !v);
            }}
          >
            {badgeLabel}
          </button>
        </div>
      )}
      <div className={styles.body}>
        <div className={styles.scene}>
          <div className={styles.glowWrap} onClick={(e) => e.stopPropagation()}>
            <div
              className={styles.glow}
              style={{ backgroundColor: palette.body }}
            />
            <BuddyCanvas
              state={state}
              onEvent={buddy.handleCanvasEvent}
              displaySize={200}
              speechOverride={speechText}
              speechControls={speechControls}
              onSpeechControlClick={speechHandler}
            />
          </div>
        </div>

        <div className={styles.info}>
          {nowPlaying?.progress != null && (
            <div className={styles.statusBubble}>
              <span className={styles.statusIcon}>
                {SIGNALS[nowPlaying.signal_type].icon}
              </span>
              <div className={styles.progressBar}>
                <div style={{ width: `${nowPlaying.progress}%` }} />
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
