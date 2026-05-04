import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { render } from "../../utils/test-utils";
import { VirtualizedChatList } from "./VirtualizedChatList";

type VirtuosoCall = {
  atBottomStateChange?: (atBottom: boolean) => void;
  followOutput?: (isAtBottom: boolean) => false | "auto" | "smooth";
  increaseViewportBy?: { top: number; bottom: number };
  skipAnimationFrameInResizeObserver?: boolean;
};

function getVirtuosoCalls(): VirtuosoCall[] {
  return (
    ((globalThis as Record<string, unknown>).__VIRTUOSO_CALLS__ as
      | VirtuosoCall[]
      | undefined) ?? []
  );
}

type Item = { key: string; text: string };

const items: Item[] = Array.from({ length: 4 }, (_, i) => ({
  key: `k-${i}`,
  text: `item-${i}`,
}));

describe("VirtualizedChatList", () => {
  beforeEach(() => {
    (globalThis as Record<string, unknown>).__VIRTUOSO_CALLS__ = [];
    vi.useRealTimers();
  });

  test("uses tighter viewport padding for streaming vs idle", () => {
    const { rerender } = render(
      <div style={{ height: 400 }}>
        <VirtualizedChatList
          items={items}
          isStreaming
          renderItem={(item) => <div>{item.text}</div>}
        />
      </div>,
    );

    const firstCall = getVirtuosoCalls().at(-1);
    expect(firstCall?.increaseViewportBy).toEqual({ top: 800, bottom: 1200 });

    rerender(
      <div style={{ height: 400 }}>
        <VirtualizedChatList
          items={items}
          isStreaming={false}
          renderItem={(item) => <div>{item.text}</div>}
        />
      </div>,
    );

    const secondCall = getVirtuosoCalls().at(-1);
    expect(secondCall?.increaseViewportBy).toEqual({ top: 1600, bottom: 2200 });
  });

  test("uses synchronous ResizeObserver measurements to reduce dynamic-height jitter", () => {
    render(
      <div style={{ height: 400 }}>
        <VirtualizedChatList
          items={items}
          isStreaming
          renderItem={(item) => <div>{item.text}</div>}
        />
      </div>,
    );

    const call = getVirtuosoCalls().at(-1);
    expect(call?.skipAnimationFrameInResizeObserver).toBe(true);
  });

  test("re-arms auto-follow when keyboard users scroll back down", async () => {
    render(
      <div style={{ height: 400 }}>
        <VirtualizedChatList
          items={items}
          isStreaming
          renderItem={(item) => <div>{item.text}</div>}
        />
      </div>,
    );

    const scroller = screen.getByTestId("chat-virtuoso-scroller");
    const call = getVirtuosoCalls().at(-1);

    Object.defineProperty(scroller, "scrollTop", {
      configurable: true,
      value: 100,
      writable: true,
    });
    fireEvent.scroll(scroller);

    fireEvent.wheel(scroller, { deltaY: -20 });
    scroller.scrollTop = 40;
    fireEvent.scroll(scroller);
    const onBottom = call?.atBottomStateChange;
    expect(onBottom).toBeDefined();
    onBottom?.(false);
    expect(screen.getByTitle("Follow stream")).toBeInTheDocument();

    fireEvent.keyDown(scroller, { key: "End" });
    onBottom?.(true);
    await waitFor(() => {
      expect(screen.queryByTitle("Follow stream")).not.toBeInTheDocument();
    });
  });

  test("does not treat Virtuoso passive upward corrections as user scroll", () => {
    render(
      <div style={{ height: 400 }}>
        <VirtualizedChatList
          items={items}
          isStreaming
          renderItem={(item) => <div>{item.text}</div>}
        />
      </div>,
    );

    const scroller = screen.getByTestId("chat-virtuoso-scroller");
    Object.defineProperty(scroller, "scrollTop", {
      configurable: true,
      value: 100,
      writable: true,
    });

    fireEvent.scroll(scroller);
    scroller.scrollTop = 40;
    fireEvent.scroll(scroller);

    expect(screen.queryByTitle("Follow stream")).not.toBeInTheDocument();
  });

  test("keeps following when dynamic height temporarily reports not at bottom", () => {
    render(
      <div style={{ height: 400 }}>
        <VirtualizedChatList
          items={items}
          isStreaming
          renderItem={(item) => <div>{item.text}</div>}
        />
      </div>,
    );

    const call = getVirtuosoCalls().at(-1);
    expect(call?.followOutput?.(false)).toBe("auto");
  });

  test("real pointer scroll-up disables follow even during suppression window", () => {
    render(
      <div style={{ height: 400 }}>
        <VirtualizedChatList
          items={items}
          isStreaming
          renderItem={(item) => <div>{item.text}</div>}
        />
      </div>,
    );

    const scroller = screen.getByTestId("chat-virtuoso-scroller");
    const call = getVirtuosoCalls().at(-1);
    Object.defineProperty(scroller, "scrollTop", {
      configurable: true,
      value: 100,
      writable: true,
    });

    fireEvent.scroll(scroller);
    expect(call?.followOutput?.(false)).toBe("auto");
    fireEvent.pointerDown(scroller);
    scroller.scrollTop = 40;
    fireEvent.scroll(scroller);

    expect(screen.getByTitle("Follow stream")).toBeInTheDocument();
    expect(call?.followOutput?.(false)).toBe(false);
  });

  test("keeps following recently changed output after streaming ends", () => {
    const { rerender } = render(
      <div style={{ height: 400 }}>
        <VirtualizedChatList
          items={items}
          isStreaming={false}
          renderItem={(item) => <div>{item.text}</div>}
        />
      </div>,
    );

    rerender(
      <div style={{ height: 400 }}>
        <VirtualizedChatList
          items={[...items, { key: "task-done", text: "task done" }]}
          isStreaming={false}
          renderItem={(item) => <div>{item.text}</div>}
        />
      </div>,
    );

    const call = getVirtuosoCalls().at(-1);
    expect(call?.followOutput?.(false)).toBe("auto");
  });

  test("does not grant post-stream follow when items are recreated without output change", () => {
    vi.useFakeTimers();
    const { rerender } = render(
      <div style={{ height: 400 }}>
        <VirtualizedChatList
          items={items}
          isStreaming={false}
          renderItem={(item) => <div>{item.text}</div>}
        />
      </div>,
    );

    const firstCall = getVirtuosoCalls().at(-1);
    expect(firstCall?.followOutput?.(false)).toBe("auto");
    vi.advanceTimersByTime(300);

    rerender(
      <div style={{ height: 400 }}>
        <VirtualizedChatList
          items={[...items]}
          isStreaming={false}
          renderItem={(item) => <div>{item.text}</div>}
        />
      </div>,
    );

    const secondCall = getVirtuosoCalls().at(-1);
    expect(secondCall?.followOutput?.(false)).toBe(false);
  });

  test("wheel inside nested content does not disable outer auto-follow", () => {
    render(
      <div style={{ height: 400 }}>
        <VirtualizedChatList
          items={items}
          isStreaming
          renderItem={(item) => <div>{item.text}</div>}
        />
      </div>,
    );

    const scroller = screen.getByTestId("chat-virtuoso-scroller");
    const nested = screen.getByText("item-1");
    const call = getVirtuosoCalls().at(-1);

    Object.defineProperty(scroller, "scrollTop", {
      configurable: true,
      value: 100,
      writable: true,
    });
    fireEvent.scroll(scroller);

    fireEvent.wheel(nested, { deltaY: -30 });

    expect(screen.queryByTitle("Follow stream")).not.toBeInTheDocument();
    expect(call?.followOutput?.(false)).toBe("auto");
  });
});
