import { describe, expect, it } from "vitest";
import { act } from "react-dom/test-utils";

import { render, screen } from "../../utils/test-utils";
import { Toolbar } from "./Toolbar";
import { createChatWithId, switchToThread } from "../../features/Chat/Thread";
import { processCompleted } from "../../features/Notifications";
import type { ProcessCompletedEvent } from "../../features/Notifications";

const threadId = "thread-with-notification";

function makeProcessCompletedEvent(): ProcessCompletedEvent {
  return {
    chat_id: threadId,
    seq: "3",
    type: "process_completed",
    process_id: "exec_sidebar",
    status: "failed",
    exit_code: 1,
    short_description: "Run sidebar test",
    mode: "background",
  };
}

describe("Toolbar notification badge", () => {
  it("shows the pending process completion count on the thread tab", () => {
    const { store } = render(<Toolbar activeTab={{ type: "dashboard" }} />);

    act(() => {
      store.dispatch(createChatWithId({ id: threadId, title: "Badge chat" }));
      const firstThreadId = store.getState().chat.open_thread_ids[0];
      if (!firstThreadId) throw new Error("missing initial test thread");
      store.dispatch(switchToThread({ id: firstThreadId }));
      store.dispatch(processCompleted(makeProcessCompletedEvent()));
    });

    expect(
      screen.getByLabelText("1 unread process notifications"),
    ).toHaveTextContent("1");
  });
});
