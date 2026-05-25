import { describe, expect, it, vi } from "vitest";
import { render, screen } from "../../utils/test-utils";
import { PlannerItem } from "./TaskWorkspace";
import type { PlannerInfo } from "./tasksSlice";
import type { ChatThreadRuntime } from "../Chat/Thread/types";

const PLANNER_ID = "planner-test-1";

const makePlanner = (waitingForCardIds?: string[]): PlannerInfo => ({
  id: PLANNER_ID,
  title: "Test Planner",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
  waitingForCardIds,
});

const makeRuntime = (sessionState?: string): ChatThreadRuntime => ({
  thread: {
    id: PLANNER_ID,
    messages: [],
    title: "Test Planner",
    model: "",
    last_user_message_id: "",
    new_chat_suggested: { wasSuggested: false },
  },
  streaming: false,
  waiting_for_response: false,
  prevent_send: false,
  error: null,
  queued_items: [],
  send_immediately: false,
  attached_images: [],
  attached_text_files: [],
  confirmation: {
    pause: false,
    pause_reasons: [],
    status: { wasInteracted: false, confirmationStatus: true },
  },
  snapshot_received: true,
  task_widget_expanded: false,
  memory_enrichment_user_touched: false,
  manual_preview_items: [],
  manual_preview_ran: false,
  session_state: sessionState,
});

const makePreloadedState = (sessionState?: string) => ({
  chat: {
    current_thread_id: PLANNER_ID,
    open_thread_ids: [PLANNER_ID],
    threads: { [PLANNER_ID]: makeRuntime(sessionState) },
    system_prompt: {},
    tool_use: "explore" as const,
    sse_refresh_requested: null,
    stream_version: 0,
  },
});

describe("PlannerItem waiting chips", () => {
  it("renders waiting card chips when session_state === 'waiting_user_input'", () => {
    const planner = makePlanner(["T-2", "T-3", "T-5"]);

    render(
      <PlannerItem
        planner={planner}
        isSelected={false}
        onSelect={vi.fn()}
        onRemove={vi.fn()}
      />,
      { preloadedState: makePreloadedState("waiting_user_input") },
    );

    expect(screen.getByText("T-2")).toBeInTheDocument();
    expect(screen.getByText("T-3")).toBeInTheDocument();
    expect(screen.getByText("T-5")).toBeInTheDocument();
  });

  it("caps chip list at 5 with '… and N more'", () => {
    const planner = makePlanner([
      "T-1",
      "T-2",
      "T-3",
      "T-4",
      "T-5",
      "T-6",
      "T-7",
      "T-8",
    ]);

    render(
      <PlannerItem
        planner={planner}
        isSelected={false}
        onSelect={vi.fn()}
        onRemove={vi.fn()}
      />,
      { preloadedState: makePreloadedState("waiting_user_input") },
    );

    expect(screen.getByText("T-1")).toBeInTheDocument();
    expect(screen.getByText("T-5")).toBeInTheDocument();
    expect(screen.queryByText("T-6")).not.toBeInTheDocument();
    expect(screen.getByText(/and 3 more/)).toBeInTheDocument();
  });

  it("does not render chips when session_state !== 'waiting_user_input'", () => {
    const planner = makePlanner(["T-2", "T-3", "T-5"]);

    render(
      <PlannerItem
        planner={planner}
        isSelected={false}
        onSelect={vi.fn()}
        onRemove={vi.fn()}
      />,
      { preloadedState: makePreloadedState("generating") },
    );

    expect(
      screen.queryByTestId(`planner-waiting-chips-${planner.id}`),
    ).not.toBeInTheDocument();
  });

  it("does not render chips when waitingForCardIds is empty", () => {
    const planner = makePlanner([]);

    render(
      <PlannerItem
        planner={planner}
        isSelected={false}
        onSelect={vi.fn()}
        onRemove={vi.fn()}
      />,
      { preloadedState: makePreloadedState("waiting_user_input") },
    );

    expect(
      screen.queryByTestId(`planner-waiting-chips-${planner.id}`),
    ).not.toBeInTheDocument();
  });

  it("does not render chips when waitingForCardIds is undefined", () => {
    const planner = makePlanner(undefined);

    render(
      <PlannerItem
        planner={planner}
        isSelected={false}
        onSelect={vi.fn()}
        onRemove={vi.fn()}
      />,
      { preloadedState: makePreloadedState("waiting_user_input") },
    );

    expect(
      screen.queryByTestId(`planner-waiting-chips-${planner.id}`),
    ).not.toBeInTheDocument();
  });
});
