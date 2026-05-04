import { createReducer, createAction } from "@reduxjs/toolkit";
import { RootState } from "../../app/store";

export type CurrentProjectInfo = {
  name: string;
  workspaceRoots?: string[];
  serverSnapshotReceived: boolean;
};

export type CurrentProjectInfoUpdate = Partial<CurrentProjectInfo>;

const initialState: CurrentProjectInfo = {
  name: "",
  serverSnapshotReceived: false,
};

export const setCurrentProjectInfo = createAction<CurrentProjectInfoUpdate>(
  "currentProjectInfo/setCurrentProjectInfo",
);

export const currentProjectInfoReducer = createReducer(
  initialState,
  (builder) => {
    builder.addCase(setCurrentProjectInfo, (state, action) => {
      return {
        ...state,
        ...action.payload,
        serverSnapshotReceived:
          action.payload.serverSnapshotReceived ?? state.serverSnapshotReceived,
      };
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
  const hasWorkspaceRoot =
    workspaceRoots !== undefined && workspaceRoots.length > 0;
  return Boolean(
    hasWorkspaceRoot ||
      state.current_project.name.trim() ||
      state.config.currentWorkspaceName?.trim(),
  );
};

export const selectHasProjectSnapshot = (state: RootState): boolean =>
  state.current_project.serverSnapshotReceived === true;
