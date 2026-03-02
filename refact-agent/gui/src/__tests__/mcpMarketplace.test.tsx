import { describe, expect, it } from "vitest";
import { render, screen, fireEvent } from "../utils/test-utils";
import { http, HttpResponse } from "msw";
import { server } from "../utils/mockServer";
import { MCPMarketplace } from "../features/MCPMarketplace";
import { ServerCard } from "../features/MCPMarketplace/ServerCard";
import type { MCPServer, MarketplaceResponse } from "../services/refact/mcpMarketplace";

const MOCK_SERVER: MCPServer = {
  id: "test-server",
  name: "Test Server",
  description: "A test MCP server for unit tests",
  publisher: "Test Publisher",
  tags: ["search", "code"],
  transport: "stdio",
  install_recipe: {
    command: "npx test-server",
    env: { API_KEY: "" },
  },
  confirmation_default: [],
};

const MOCK_RESPONSE: MarketplaceResponse = {
  servers: [MOCK_SERVER],
  source: "local",
};

const PRELOADED_STATE = {
  config: {
    apiKey: "test",
    lspPort: 8001,
    themeProps: {},
    host: "vscode" as const,
    addressURL: "Refact",
  },
};

describe("ServerCard", () => {
  it("renders server name, publisher and description", () => {
    render(
      <ServerCard
        server={MOCK_SERVER}
        isInstalled={false}
        isInstalling={false}
        onInstall={() => undefined}
        onViewDetail={() => undefined}
      />,
    );
    expect(screen.getByText("Test Server")).toBeDefined();
    expect(screen.getByText("Test Publisher")).toBeDefined();
    expect(screen.getByText("A test MCP server for unit tests")).toBeDefined();
  });

  it("renders Install button when not installed", () => {
    render(
      <ServerCard
        server={MOCK_SERVER}
        isInstalled={false}
        isInstalling={false}
        onInstall={() => undefined}
        onViewDetail={() => undefined}
      />,
    );
    expect(screen.getByRole("button", { name: /install/i })).toBeDefined();
    expect(screen.queryByText("Installed")).toBeNull();
  });

  it("renders Installed text when installed", () => {
    render(
      <ServerCard
        server={MOCK_SERVER}
        isInstalled={true}
        isInstalling={false}
        onInstall={() => undefined}
        onViewDetail={() => undefined}
      />,
    );
    expect(screen.getByText("Installed")).toBeDefined();
    expect(screen.queryByRole("button", { name: /^install$/i })).toBeNull();
  });

  it("renders tags as badges", () => {
    render(
      <ServerCard
        server={MOCK_SERVER}
        isInstalled={false}
        isInstalling={false}
        onInstall={() => undefined}
        onViewDetail={() => undefined}
      />,
    );
    expect(screen.getByText("search")).toBeDefined();
    expect(screen.getByText("code")).toBeDefined();
  });

  it("calls onInstall with server when Install button clicked", () => {
    const calledWith: MCPServer[] = [];
    render(
      <ServerCard
        server={MOCK_SERVER}
        isInstalled={false}
        isInstalling={false}
        onInstall={(s) => { calledWith.push(s); }}
        onViewDetail={() => undefined}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /install/i }));
    expect(calledWith.length).toBe(1);
    expect(calledWith[0]?.id).toBe("test-server");
  });
});

describe("MCPMarketplace", () => {
  it("renders marketplace page with server cards from API", async () => {
    server.use(
      http.get("http://127.0.0.1:8001/v1/mcp/marketplace", () => {
        return HttpResponse.json(MOCK_RESPONSE);
      }),
      http.get("http://127.0.0.1:8001/v1/mcp/marketplace/installed", () => {
        return HttpResponse.json({ installed: [] });
      }),
    );

    render(
      <MCPMarketplace
        host="vscode"
        tabbed={false}
        backFromMarketplace={() => undefined}
      />,
      { preloadedState: PRELOADED_STATE },
    );

    expect(await screen.findByText("Test Server")).toBeDefined();
    expect(screen.getByText("MCP Marketplace")).toBeDefined();
  });

  it("filters servers by search query", async () => {
    const secondServer: MCPServer = {
      ...MOCK_SERVER,
      id: "other-server",
      name: "Other Service",
      description: "Another service",
      tags: ["database"],
    };
    server.use(
      http.get("http://127.0.0.1:8001/v1/mcp/marketplace", () => {
        return HttpResponse.json({ servers: [MOCK_SERVER, secondServer], source: "local" });
      }),
      http.get("http://127.0.0.1:8001/v1/mcp/marketplace/installed", () => {
        return HttpResponse.json({ installed: [] });
      }),
    );

    render(
      <MCPMarketplace
        host="vscode"
        tabbed={false}
        backFromMarketplace={() => undefined}
      />,
      { preloadedState: PRELOADED_STATE },
    );

    await screen.findByText("Test Server");
    expect(screen.getByText("Other Service")).toBeDefined();

    const searchInput = screen.getByPlaceholderText("Search servers…");
    fireEvent.change(searchInput, { target: { value: "Other" } });

    expect(screen.queryByText("Test Server")).toBeNull();
    expect(screen.getByText("Other Service")).toBeDefined();
  });

  it("shows installed indicator for installed servers", async () => {
    server.use(
      http.get("http://127.0.0.1:8001/v1/mcp/marketplace", () => {
        return HttpResponse.json(MOCK_RESPONSE);
      }),
      http.get("http://127.0.0.1:8001/v1/mcp/marketplace/installed", () => {
        return HttpResponse.json({
          installed: [{ id: "test-server", name: "Test Server", config_path: "/tmp/test.yaml" }],
        });
      }),
    );

    render(
      <MCPMarketplace
        host="vscode"
        tabbed={false}
        backFromMarketplace={() => undefined}
      />,
      { preloadedState: PRELOADED_STATE },
    );

    await screen.findByText("Test Server");
    expect(screen.getByText("Installed")).toBeDefined();
  });
});
