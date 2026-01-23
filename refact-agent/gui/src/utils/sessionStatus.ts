import type { StatusDotState } from "../components/StatusDot";

export type SessionState =
  | "idle"
  | "generating"
  | "executing_tools"
  | "paused"
  | "waiting_ide"
  | "error";

export function getStatusFromSessionState(
  sessionState?: string | null,
): StatusDotState {
  if (sessionState === "generating" || sessionState === "executing_tools") {
    return "streaming";
  }
  if (sessionState === "paused" || sessionState === "waiting_ide") {
    return "paused";
  }
  if (sessionState === "error") {
    return "error";
  }
  return "idle";
}

export function getStatusTooltip(sessionState?: string | null): string {
  if (sessionState === "generating" || sessionState === "executing_tools") {
    return "Generating response...";
  }
  if (sessionState === "paused" || sessionState === "waiting_ide") {
    return "Waiting for confirmation";
  }
  if (sessionState === "error") {
    return "An error occurred";
  }
  return "Idle";
}
