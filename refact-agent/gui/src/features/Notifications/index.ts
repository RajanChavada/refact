import type { NotificationsState } from "./notificationsSlice";

export {
  notificationsSlice,
  processCompleted,
  clearProcessCompletions,
} from "./notificationsSlice";
export type {
  NotificationsState,
  ProcessCompletedEvent,
} from "./notificationsSlice";

export const selectProcessCompletions = (state: {
  notifications: NotificationsState;
}) => state.notifications.processCompletions;
