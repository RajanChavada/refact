import { describe, expect, it } from "vitest";

import {
  currentProjectInfoReducer,
  selectHasActiveProject,
  selectHasProjectSnapshot,
  setCurrentProjectInfo,
} from "../features/Chat/currentProject";
import type { RootState } from "../app/store";

function rootStateWithCurrentProject(
  currentProject: RootState["current_project"],
): RootState {
  return {
    current_project: currentProject,
    config: {},
    chat: {
      current_thread_id: "",
      threads: {},
    },
  } as RootState;
}

describe("current project state", () => {
  it("preserves server snapshot readiness when a local project update omits it", () => {
    const serverSnapshotState = currentProjectInfoReducer(
      undefined,
      setCurrentProjectInfo({
        name: "refact",
        workspaceRoots: ["/workspace/refact"],
        serverSnapshotReceived: true,
      }),
    );

    const localUpdateState = currentProjectInfoReducer(
      serverSnapshotState,
      setCurrentProjectInfo({
        name: "refact-renamed",
      }),
    );

    expect(localUpdateState).toEqual({
      name: "refact-renamed",
      workspaceRoots: ["/workspace/refact"],
      serverSnapshotReceived: true,
    });
  });

  it("treats an explicit empty server workspace as a received snapshot", () => {
    const state = currentProjectInfoReducer(
      undefined,
      setCurrentProjectInfo({
        name: "",
        workspaceRoots: [],
        serverSnapshotReceived: true,
      }),
    );
    const rootState = rootStateWithCurrentProject(state);

    expect(selectHasProjectSnapshot(rootState)).toBe(true);
    expect(selectHasActiveProject(rootState)).toBe(false);
  });
});
