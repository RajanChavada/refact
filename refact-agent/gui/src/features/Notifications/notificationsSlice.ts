import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import type { ChatEventEnvelope } from "../../services/refact/chatSubscription";

export type ProcessCompletedEvent = Extract<
  ChatEventEnvelope,
  { type: "process_completed" }
>;

export type NotificationsState = {
  processCompletions: ProcessCompletedEvent[];
};

const MAX_PROCESS_COMPLETIONS = 20;

const initialState: NotificationsState = {
  processCompletions: [],
};

export const notificationsSlice = createSlice({
  name: "notifications",
  initialState,
  reducers: {
    processCompleted: (state, action: PayloadAction<ProcessCompletedEvent>) => {
      const index = state.processCompletions.findIndex(
        (event) =>
          event.chat_id === action.payload.chat_id &&
          event.seq === action.payload.seq,
      );
      if (index >= 0) {
        state.processCompletions[index] = action.payload;
      } else {
        state.processCompletions.push(action.payload);
      }
      if (state.processCompletions.length > MAX_PROCESS_COMPLETIONS) {
        state.processCompletions.splice(
          0,
          state.processCompletions.length - MAX_PROCESS_COMPLETIONS,
        );
      }
    },
    clearProcessCompletions: (state) => {
      state.processCompletions = [];
    },
  },
});

export const { processCompleted, clearProcessCompletions } =
  notificationsSlice.actions;
