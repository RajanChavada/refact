import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { Provider } from "react-redux";
import { http, HttpResponse } from "msw";

import { setUpStore } from "../app/store";
import { useSidebarSubscription } from "../hooks/useSidebarSubscription";
import { server } from "../utils/mockServer";
import { setCurrentProjectInfo } from "../features/Chat/currentProject";

function envelope(seq: number, event: Record<string, unknown>) {
  return {
    protocol_version: 2,
    seq,
    subscription_id: "test-sidebar",
    event,
  };
}

function sectionSnapshot(
  seq: number,
  section: "workspace" | "chats" | "tasks" | "buddy",
  snapshot: Record<string, unknown>,
  status: "ready" | "error" = "ready",
  error?: string,
) {
  return envelope(seq, {
    type: "section_snapshot",
    section,
    status,
    snapshot,
    ...(error ? { error } : {}),
  });
}

function sidebarSnapshotHandler(...events: Record<string, unknown>[]) {
  return http.get("http://127.0.0.1:8001/v1/sidebar/subscribe", () => {
    const encoder = new TextEncoder();
    const stream = new ReadableStream({
      start(controller) {
        for (const event of events) {
          controller.enqueue(
            encoder.encode(`data: ${JSON.stringify(event)}\n\n`),
          );
        }
      },
    });

    return new HttpResponse(stream, {
      headers: {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache",
        Connection: "keep-alive",
      },
    });
  });
}

function renderSidebarSubscription(
  preloadedState: Parameters<typeof setUpStore>[0] = {},
) {
  const store = setUpStore({
    config: {
      apiKey: "test",
      lspPort: 8001,
      themeProps: {},
      host: "vscode",
    },
    ...preloadedState,
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <Provider store={store}>{children}</Provider>
  );

  renderHook(() => useSidebarSubscription(), { wrapper });

  return store;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("useSidebarSubscription", () => {
  it("keeps local project info while waiting for an explicit workspace snapshot", async () => {
    server.use(
      sidebarSnapshotHandler(
        sectionSnapshot(0, "chats", { trajectories: [] }),
        sectionSnapshot(1, "tasks", { tasks: [] }),
        sectionSnapshot(2, "buddy", { buddy: null }),
      ),
    );

    const store = renderSidebarSubscription({
      current_project: {
        name: "local-refact",
        workspaceRoots: ["/local/refact"],
      },
    });

    await waitFor(() => {
      expect(store.getState().sidebar.sections.chats.status).toBe("ready");
      expect(store.getState().sidebar.sections.tasks.status).toBe("ready");
    });

    expect(store.getState().current_project).toEqual({
      name: "local-refact",
      workspaceRoots: ["/local/refact"],
    });
    expect(store.getState().sidebar.sections.workspace.status).toBe("loading");
  });

  it("accepts an explicit empty server workspace snapshot as loaded", async () => {
    server.use(
      sidebarSnapshotHandler(
        sectionSnapshot(0, "workspace", { workspace_roots: [] }),
        sectionSnapshot(1, "chats", { trajectories: [] }),
        sectionSnapshot(2, "tasks", { tasks: [] }),
        sectionSnapshot(3, "buddy", { buddy: null }),
      ),
    );

    const store = renderSidebarSubscription();

    await waitFor(() => {
      expect(store.getState().current_project).toEqual({
        name: "",
        workspaceRoots: [],
      });
      expect(store.getState().sidebar.sections.workspace.status).toBe("ready");
      expect(store.getState().sidebar.sections.chats.status).toBe("ready");
      expect(store.getState().sidebar.sections.tasks.status).toBe("ready");
      expect(store.getState().sidebar.sections.buddy.status).toBe("ready");
    });
  });

  it("keeps history loading false after an empty chat snapshot", async () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation((key) =>
      key === "refact-trajectories-migrated" ? "true" : null,
    );
    server.use(
      sidebarSnapshotHandler(
        sectionSnapshot(0, "workspace", {
          workspace_roots: ["/workspace/refact"],
        }),
        sectionSnapshot(1, "chats", { trajectories: [] }),
      ),
    );

    const store = renderSidebarSubscription();

    await waitFor(() => {
      expect(store.getState().history.isLoading).toBe(false);
    });
  });

  it("does not return to project loading when local IDE project info matches the server snapshot", async () => {
    server.use(
      sidebarSnapshotHandler(
        sectionSnapshot(0, "workspace", {
          workspace_roots: ["/workspace/refact"],
        }),
      ),
    );

    const store = renderSidebarSubscription();

    await waitFor(() => {
      expect(store.getState().sidebar.sections.workspace.status).toBe("ready");
    });

    store.dispatch(
      setCurrentProjectInfo({
        name: "refact",
        workspaceRoots: ["/workspace/refact"],
      }),
    );

    expect(store.getState().sidebar.sections.workspace.status).toBe("ready");
  });

  it("tracks changed local IDE project info separately from sidebar section status", async () => {
    server.use(
      sidebarSnapshotHandler(
        sectionSnapshot(0, "workspace", {
          workspace_roots: ["/workspace/refact"],
        }),
      ),
    );

    const store = renderSidebarSubscription();

    await waitFor(() => {
      expect(store.getState().sidebar.sections.workspace.status).toBe("ready");
    });

    store.dispatch(
      setCurrentProjectInfo({
        name: "other-project",
        workspaceRoots: ["/workspace/other-project"],
      }),
    );

    expect(store.getState().current_project).toEqual({
      name: "other-project",
      workspaceRoots: ["/workspace/other-project"],
    });
    expect(store.getState().sidebar.sections.workspace.status).toBe("ready");
  });
});
