import React from "react";
import { describe, expect, it } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { Provider } from "react-redux";
import { http, HttpResponse } from "msw";

import { setUpStore } from "../app/store";
import { setBackendStatus } from "../features/Connection";
import { useGetConfiguredProvidersQuery } from "../hooks/useProvidersQuery";
import { providersApi } from "../services/refact";
import { server } from "../utils/mockServer";

function createWrapper() {
  const store = setUpStore({
    config: {
      apiKey: "test",
      lspPort: 8001,
      themeProps: {},
      host: "vscode",
    },
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <Provider store={store}>{children}</Provider>
  );

  return { store, wrapper };
}

describe("useGetConfiguredProvidersQuery", () => {
  it("skips providers while backend is offline and fetches after it becomes online", async () => {
    let providersRequests = 0;
    server.use(
      http.get("http://127.0.0.1:8001/v1/providers", () => {
        providersRequests += 1;
        return HttpResponse.json({ providers: [] });
      }),
    );

    const { store, wrapper } = createWrapper();
    store.dispatch(setBackendStatus({ status: "offline" }));

    const { result } = renderHook(() => useGetConfiguredProvidersQuery(), {
      wrapper,
    });

    expect(result.current.isUninitialized).toBe(true);
    expect(providersRequests).toBe(0);

    store.dispatch(setBackendStatus({ status: "online" }));

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });
    expect(providersRequests).toBe(1);

    store.dispatch(providersApi.util.resetApiState());
  });
});
