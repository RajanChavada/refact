import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import { RootState } from "../../app/store";

export type DiffBox = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type BrowserTabInfo = {
  id: string;
  url: string;
  title: string;
};

export type BrowserFrame = {
  mime: string;
  data: string;
  diff_boxes: DiffBox[];
};

export type BrowserRuntime = {
  runtime_id: string;
  connected: boolean;
  active_tab: string | null;
  url: string | null;
  title: string | null;
  tabs: BrowserTabInfo[];
  latest_frame: BrowserFrame | null;
  picker_active: boolean;
  attach_screenshot_on_send: boolean;
};

export type BrowserState = {
  runtimes: Record<string, BrowserRuntime | undefined>;
};

const initialState: BrowserState = {
  runtimes: {},
};

export const browserSlice = createSlice({
  name: "browser",
  initialState,
  reducers: {
    setBrowserRuntime(
      state,
      action: PayloadAction<{ chatId: string; runtime: BrowserRuntime }>,
    ) {
      state.runtimes[action.payload.chatId] = action.payload.runtime;
    },
    updateBrowserStatus(
      state,
      action: PayloadAction<{
        chatId: string;
        connected: boolean;
        url?: string | null;
        title?: string | null;
      }>,
    ) {
      const rt = state.runtimes[action.payload.chatId];
      if (rt) {
        rt.connected = action.payload.connected;
        if (action.payload.url !== undefined) rt.url = action.payload.url;
        if (action.payload.title !== undefined)
          rt.title = action.payload.title;
      }
    },
    updateBrowserFrame(
      state,
      action: PayloadAction<{ chatId: string; frame: BrowserFrame }>,
    ) {
      const rt = state.runtimes[action.payload.chatId];
      if (rt) {
        rt.latest_frame = action.payload.frame;
      }
    },
    removeBrowserRuntime(state, action: PayloadAction<{ chatId: string }>) {
      const { [action.payload.chatId]: _, ...rest } = state.runtimes;
      state.runtimes = rest;
    },
    setPickerActive(
      state,
      action: PayloadAction<{ chatId: string; active: boolean }>,
    ) {
      const rt = state.runtimes[action.payload.chatId];
      if (rt) {
        rt.picker_active = action.payload.active;
      }
    },
    toggleAttachScreenshotOnSend(
      state,
      action: PayloadAction<{ chatId: string }>,
    ) {
      const rt = state.runtimes[action.payload.chatId];
      if (rt) {
        rt.attach_screenshot_on_send = !rt.attach_screenshot_on_send;
      }
    },
  },
});

export const {
  setBrowserRuntime,
  updateBrowserStatus,
  updateBrowserFrame,
  removeBrowserRuntime,
  setPickerActive,
  toggleAttachScreenshotOnSend,
} = browserSlice.actions;

export const selectBrowserRuntime = (
  state: RootState,
  chatId: string,
): BrowserRuntime | undefined => state.browser.runtimes[chatId];

export const selectBrowserRuntimes = (state: RootState) =>
  state.browser.runtimes;
