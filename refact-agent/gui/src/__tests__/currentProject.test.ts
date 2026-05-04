import { describe, expect, it } from "vitest";
import {
  currentProjectInfoReducer,
  resetSidebarReadiness,
  setCurrentProjectInfo,
} from "../features/Chat/currentProject";

describe("currentProjectInfoReducer", () => {
  it("preserves workspace roots when the same project update omits roots", () => {
    let state = currentProjectInfoReducer(
      undefined,
      setCurrentProjectInfo({
        name: "refact",
        workspaceRoots: ["/tmp/a/refact"],
      }),
    );

    state = currentProjectInfoReducer(
      state,
      setCurrentProjectInfo({ name: "refact" }),
    );

    expect(state.workspaceRoots).toEqual(["/tmp/a/refact"]);
  });

  it("uses workspace roots, not matching names, for known project identity", () => {
    let state = currentProjectInfoReducer(
      undefined,
      setCurrentProjectInfo({
        name: "refact",
        workspaceRoots: ["/tmp/a/refact"],
      }),
    );

    state = currentProjectInfoReducer(
      state,
      setCurrentProjectInfo({
        name: "refact",
        workspaceRoots: ["/tmp/b/refact"],
      }),
    );

    expect(state.workspaceRoots).toEqual(["/tmp/b/refact"]);
  });

  it("does not reset project identity for the compatibility readiness reset action", () => {
    const state = currentProjectInfoReducer(
      {
        name: "refact",
        workspaceRoots: ["/tmp/a/refact"],
      },
      resetSidebarReadiness(),
    );

    expect(state).toEqual({
      name: "refact",
      workspaceRoots: ["/tmp/a/refact"],
    });
  });
});
