import { createReducer, createAction } from "@reduxjs/toolkit";
import { RootState } from "../../app/store";

export type CurrentProjectInfo = {
  name: string;
  workspaceRoots?: string[];
};

const initialState: CurrentProjectInfo = {
  name: "",
};

export const setCurrentProjectInfo = createAction<CurrentProjectInfo>(
  "currentProjectInfo/setCurrentProjectInfo",
);
export const resetSidebarReadiness = createAction(
  "currentProjectInfo/resetSidebarReadiness",
);

function shouldPreserveWorkspaceRoots(
  state: CurrentProjectInfo,
  next: CurrentProjectInfo,
): boolean {
  if (!state.workspaceRoots || next.workspaceRoots !== undefined) return false;

  const nextName = next.name.trim();
  if (!nextName) return false;

  return nextName === state.name;
}

export const currentProjectInfoReducer = createReducer(
  initialState,
  (builder) => {
    builder
      .addCase(setCurrentProjectInfo, (state, action) => {
        const next = action.payload;
        const nextRoots =
          next.workspaceRoots ??
          (shouldPreserveWorkspaceRoots(state, next)
            ? state.workspaceRoots
            : undefined);

        state.name = next.name;
        if (nextRoots !== undefined) {
          state.workspaceRoots = nextRoots;
        } else {
          delete state.workspaceRoots;
        }
      })
      .addCase(resetSidebarReadiness, () => {
        // Kept as a no-op compatibility action for IDE/config messages.
      });
  },
);

export const selectThreadProjectOrCurrentProject = (state: RootState) => {
  const threadId = state.chat.current_thread_id;
  const runtime = threadId ? state.chat.threads[threadId] : undefined;
  if (!runtime) {
    return state.current_project.name;
  }
  const thread = runtime.thread;
  if (thread.integration?.project) {
    return thread.integration.project;
  }
  return thread.project_name ?? state.current_project.name;
};

export const selectHasActiveProject = (state: RootState): boolean => {
  const workspaceRoots = state.current_project.workspaceRoots;
  if (workspaceRoots !== undefined) {
    return workspaceRoots.length > 0;
  }

  return Boolean(
    state.current_project.name.trim() ||
      state.config.currentWorkspaceName?.trim(),
  );
};
