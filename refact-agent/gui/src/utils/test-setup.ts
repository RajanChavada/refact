import { beforeAll, afterEach, afterAll, vi } from "vitest";
import {
  stubResizeObserver,
  cleanup,
  stubIntersectionObserver,
} from "./test-utils";
import MatchMediaMock from "vitest-matchmedia-mock";
import React from "react";
const matchMediaMock = new MatchMediaMock();

vi.mock("react-virtuoso", () => ({
  Virtuoso: ({
    data,
    itemContent,
    components,
  }: {
    data?: unknown[];
    itemContent: (index: number, item: unknown) => React.ReactNode;
    components?: { Footer?: React.ComponentType };
  }) =>
    React.createElement(
      "div",
      null,
      ...(data ?? []).map((item, i) => itemContent(i, item)),
      components?.Footer ? React.createElement(components.Footer) : null,
    ),
}));

(globalThis as Record<string, unknown>).__REFACT_LSP_PORT__ = 8001;

beforeAll(() => {
  stubResizeObserver();
  stubIntersectionObserver();
  Element.prototype.scrollIntoView = vi.fn();

  // Mock localStorage for tests
  const storage = new Map<string, string>();
  const localStorageMock: Storage = {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => {
      storage.set(key, value);
    },
    removeItem: (key: string) => {
      storage.delete(key);
    },
    clear: () => {
      storage.clear();
    },
    key: (index: number) => Array.from(storage.keys())[index] ?? null,
    get length() {
      return storage.size;
    },
  };
  global.localStorage = localStorageMock;
});

afterEach(() => {
  cleanup();
});

afterAll(() => {
  matchMediaMock.destroy();
});

vi.mock("lottie-react", () => {
  return {
    default: vi.fn(),
    useLottie: vi.fn(() => {
      return {
        View: React.createElement("div"),
        playSegments: vi.fn(),
      };
    }),
  };
});
