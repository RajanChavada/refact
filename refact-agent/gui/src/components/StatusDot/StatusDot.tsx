import React from "react";
import { HoverCard, Text } from "@radix-ui/themes";
import styles from "./StatusDot.module.css";

export type StatusDotState =
  | "streaming"
  | "paused"
  | "error"
  | "idle"
  | "unread"
  | "completed";

export interface StatusDotProps {
  state: StatusDotState;
  size?: "small" | "medium";
  tooltipText?: string;
}

const STATE_TOOLTIPS: Record<StatusDotState, string> = {
  streaming: "Generating response...",
  paused: "Waiting for confirmation",
  error: "An error occurred",
  idle: "Idle",
  unread: "Unread messages",
  completed: "Completed",
};

const STATE_CLASS_MAP: Record<StatusDotState, string> = {
  streaming: styles.streaming,
  paused: styles.paused,
  error: styles.error,
  idle: styles.idle,
  unread: styles.unread,
  completed: styles.completed,
};

export const StatusDot: React.FC<StatusDotProps> = ({
  state,
  size = "small",
  tooltipText,
}) => {
  const sizeClass = size === "small" ? styles.small : styles.medium;
  const stateClass = STATE_CLASS_MAP[state];
  const tooltip = tooltipText ?? STATE_TOOLTIPS[state];

  return (
    <HoverCard.Root openDelay={200} closeDelay={100}>
      <HoverCard.Trigger>
        <div
          className={`${styles.dot} ${sizeClass} ${stateClass}`}
          aria-label={tooltip}
        />
      </HoverCard.Trigger>
      <HoverCard.Content size="1" side="top" align="center">
        <Text as="p" size="1">
          {tooltip}
        </Text>
      </HoverCard.Content>
    </HoverCard.Root>
  );
};
