import type { TrajectoryMeta, TrajectoryEvent } from "./trajectories";
import type { TaskMeta, TaskBoard } from "./tasks";
import type { BuddySnapshot, BuddySSEEvent } from "../../features/Buddy/types";

export type { TrajectoryMeta, TrajectoryEvent };

export type TaskEvent =
  | { type: "snapshot"; tasks: TaskMeta[] }
  | { type: "task_created"; task_id: string; meta: TaskMeta }
  | { type: "task_updated"; task_id: string; meta: TaskMeta }
  | { type: "task_deleted"; task_id: string }
  | { type: "board_changed"; task_id: string; rev: number; board: TaskBoard };

export type NotificationEvent =
  | {
      type: "task_done";
      chat_id: string;
      tool_call_id: string;
      summary: string;
      knowledge_path?: string;
    }
  | {
      type: "ask_questions";
      chat_id: string;
      tool_call_id: string;
      questions: {
        id: string;
        type: string;
        text: string;
        options?: string[];
      }[];
    };

export type SidebarSection = "workspace" | "chats" | "tasks" | "buddy";
export type SidebarSectionStatus = "ready" | "error";
export type BuddySnapshotPayload = BuddySnapshot | null;

export type SidebarSectionSnapshot =
  | { workspace_roots: string[] }
  | { trajectories: TrajectoryMeta[] }
  | { tasks: TaskMeta[] }
  | { buddy: BuddySnapshotPayload };

export type SidebarSectionUpdate = TrajectoryEvent | TaskEvent | BuddySSEEvent;

export type SidebarEvent =
  | {
      type: "section_snapshot";
      section: SidebarSection;
      status: SidebarSectionStatus;
      snapshot: SidebarSectionSnapshot;
      elapsed_ms?: number;
      error?: string;
    }
  | {
      type: "section_update";
      section: SidebarSection;
      update: SidebarSectionUpdate;
    }
  | {
      type: "notification";
      notification: NotificationEvent;
    };

export type SidebarEventEnvelope = {
  protocol_version: 2;
  seq: number;
  subscription_id: string;
  event: SidebarEvent;
};

export type SidebarSubscriptionCallbacks = {
  onEvent: (event: SidebarEventEnvelope) => void;
  onError: (error: Error) => void;
  onConnected?: () => void;
  onDisconnected?: () => void;
};

function hasArrayProperty(obj: Record<string, unknown>, key: string): boolean {
  return Array.isArray(obj[key]);
}

function isValidTrajectoryEvent(obj: Record<string, unknown>): boolean {
  return typeof obj.type === "string" && typeof obj.id === "string";
}

function isValidTaskEvent(obj: Record<string, unknown>): boolean {
  if (typeof obj.type !== "string") return false;
  if (obj.type === "snapshot") return Array.isArray(obj.tasks);
  if (obj.type === "task_deleted") return typeof obj.task_id === "string";
  if (obj.type === "board_changed") {
    return typeof obj.task_id === "string" && obj.board !== undefined;
  }
  return typeof obj.task_id === "string" && obj.meta !== undefined;
}

function isValidNotificationEvent(obj: Record<string, unknown>): boolean {
  if (typeof obj.type !== "string") return false;
  if (typeof obj.chat_id !== "string") return false;
  if (typeof obj.tool_call_id !== "string") return false;

  if (obj.type === "task_done") {
    return typeof obj.summary === "string";
  }

  if (obj.type === "ask_questions") {
    return Array.isArray(obj.questions);
  }

  return false;
}

function isValidSection(value: unknown): value is SidebarSection {
  return (
    value === "workspace" ||
    value === "chats" ||
    value === "tasks" ||
    value === "buddy"
  );
}

function isValidSectionStatus(value: unknown): value is SidebarSectionStatus {
  return value === "ready" || value === "error";
}

function isValidSectionSnapshot(
  section: SidebarSection,
  snapshot: unknown,
): snapshot is SidebarSectionSnapshot {
  if (typeof snapshot !== "object" || snapshot === null) return false;
  const obj = snapshot as Record<string, unknown>;

  if (section === "workspace") return hasArrayProperty(obj, "workspace_roots");
  if (section === "chats") return hasArrayProperty(obj, "trajectories");
  if (section === "tasks") return hasArrayProperty(obj, "tasks");
  return "buddy" in obj;
}

function isValidSectionUpdate(
  section: SidebarSection,
  update: unknown,
): update is SidebarSectionUpdate {
  if (typeof update !== "object" || update === null) return false;
  const obj = update as Record<string, unknown>;

  if (section === "chats") return isValidTrajectoryEvent(obj);
  if (section === "tasks") return isValidTaskEvent(obj);
  if (section === "buddy") return typeof obj.event_type === "string";
  return false;
}

function isValidSidebarEvent(event: unknown): event is SidebarEvent {
  if (typeof event !== "object" || event === null) return false;
  const obj = event as Record<string, unknown>;
  if (typeof obj.type !== "string") return false;

  if (obj.type === "section_snapshot") {
    if (!isValidSection(obj.section)) return false;
    if (!isValidSectionStatus(obj.status)) return false;
    if (!isValidSectionSnapshot(obj.section, obj.snapshot)) return false;
    if (obj.elapsed_ms !== undefined && typeof obj.elapsed_ms !== "number") {
      return false;
    }
    if (obj.error !== undefined && typeof obj.error !== "string") return false;
    return true;
  }

  if (obj.type === "section_update") {
    if (!isValidSection(obj.section)) return false;
    return isValidSectionUpdate(obj.section, obj.update);
  }

  if (obj.type === "notification") {
    if (typeof obj.notification !== "object" || obj.notification === null) {
      return false;
    }
    return isValidNotificationEvent(
      obj.notification as Record<string, unknown>,
    );
  }

  return false;
}

function isValidSidebarEventEnvelope(
  data: unknown,
): data is SidebarEventEnvelope {
  if (typeof data !== "object" || data === null) return false;
  const obj = data as Record<string, unknown>;
  if (obj.protocol_version !== 2) return false;
  if (typeof obj.seq !== "number") return false;
  if (typeof obj.subscription_id !== "string") return false;
  return isValidSidebarEvent(obj.event);
}

const IDLE_TIMEOUT_MS = 30_000;

export function subscribeToSidebarEvents(
  port: number,
  apiKey: string | null,
  callbacks: SidebarSubscriptionCallbacks,
): () => void {
  const url = `http://127.0.0.1:${port}/v1/sidebar/subscribe`;
  const abortController = new AbortController();
  const state = { connected: false, lastSeq: -1, aborted: false };
  let idleTimer: ReturnType<typeof setTimeout> | null = null;

  const headers: Record<string, string> = {};
  if (apiKey) {
    headers.Authorization = `Bearer ${apiKey}`;
  }

  const resetIdleTimer = () => {
    if (idleTimer) clearTimeout(idleTimer);
    idleTimer = setTimeout(() => {
      abortController.abort();
    }, IDLE_TIMEOUT_MS);
  };

  const cleanup = () => {
    if (idleTimer) {
      clearTimeout(idleTimer);
      idleTimer = null;
    }
    if (!state.aborted) {
      state.aborted = true;
      abortController.abort();
    }
    if (state.connected) {
      state.connected = false;
      callbacks.onDisconnected?.();
    }
  };

  void fetch(url, {
    method: "GET",
    headers,
    signal: abortController.signal,
  })
    .then(async (response) => {
      if (!response.ok) {
        throw new Error(`SSE connection failed: ${response.status}`);
      }
      if (!response.body) {
        throw new Error("Response body is null");
      }

      state.connected = true;
      callbacks.onConnected?.();
      resetIdleTimer();

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = "";

      try {
        for (;;) {
          const { done, value } = await reader.read();
          if (done) break;

          resetIdleTimer();
          buffer += decoder.decode(value, { stream: true });
          buffer = buffer.replace(/\r\n/g, "\n").replace(/\r/g, "\n");

          const blocks = buffer.split("\n\n");
          buffer = blocks.pop() ?? "";

          for (const block of blocks) {
            if (!block.trim()) continue;
            if (block.startsWith(":")) continue;

            const dataLines: string[] = [];
            for (const rawLine of block.split("\n")) {
              if (rawLine.startsWith(":")) continue;
              if (!rawLine.startsWith("data:")) continue;
              dataLines.push(rawLine.slice(5).replace(/^\s*/, ""));
            }

            if (dataLines.length === 0) continue;

            const dataStr = dataLines.join("\n");
            if (dataStr === "[DONE]") continue;

            let parsed: unknown;
            try {
              parsed = JSON.parse(dataStr);
            } catch (e) {
              const msg = e instanceof Error ? e.message : "JSON parse error";
              throw new Error(`Parse error: ${msg}`);
            }

            if (!isValidSidebarEventEnvelope(parsed)) {
              throw new Error("Invalid sidebar v2 event structure");
            }

            if (state.lastSeq >= 0 && parsed.seq !== state.lastSeq + 1) {
              throw new Error(
                `Seq gap: expected ${state.lastSeq + 1}, got ${parsed.seq}`,
              );
            }
            state.lastSeq = parsed.seq;

            callbacks.onEvent(parsed);
          }
        }
      } finally {
        await reader.cancel().catch(() => {
          // Ignore cancel errors - connection already closed
        });
      }

      cleanup();
    })
    .catch((err: unknown) => {
      const error = err as Error;
      if (error.name !== "AbortError") {
        callbacks.onError(error);
      }
      cleanup();
    });

  return cleanup;
}
