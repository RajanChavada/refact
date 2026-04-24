import { beforeEach, describe, expect, test } from "vitest";
import { render } from "../../utils/test-utils";
import { VirtualizedChatList } from "./VirtualizedChatList";

type VirtuosoCall = {
  atBottomStateChange?: (atBottom: boolean) => void;
  increaseViewportBy?: { top: number; bottom: number };
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
});
