import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { RootState } from "../../app/store";

export type MCPServer = {
  id: string;
  name: string;
  description: string;
  publisher: string;
  tags: string[];
  icon_url?: string;
  homepage?: string;
  transport: "stdio" | "http" | "sse";
  install_recipe: {
    command?: string;
    url?: string;
    env?: Record<string, string>;
    headers?: Record<string, string>;
  };
  confirmation_default: string[];
};

export type MarketplaceResponse = {
  servers: MCPServer[];
  source: "remote" | "local" | "merged";
};

export type InstallRequest = {
  server_id: string;
  config_overrides?: {
    env?: Record<string, string>;
    headers?: Record<string, string>;
  };
};

export type InstallResponse = {
  installed: boolean;
  config_path: string;
};

export type InstalledServer = {
  id: string;
  name: string;
  config_path: string;
};

export type InstalledResponse = {
  installed: InstalledServer[];
};

export const mcpMarketplaceApi = createApi({
  reducerPath: "mcpMarketplaceApi",
  tagTypes: ["MarketplaceServers", "InstalledServers"],
  baseQuery: fetchBaseQuery({
    prepareHeaders: (headers, { getState }) => {
      const token = (getState() as RootState).config.apiKey;
      if (token) {
        headers.set("Authorization", `Bearer ${token}`);
      }
      return headers;
    },
  }),
  endpoints: (builder) => ({
    getMarketplace: builder.query<MarketplaceResponse, undefined>({
      queryFn: async (_arg, api, _opts, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort;
        const result = await baseQuery({
          url: `http://127.0.0.1:${port}/v1/mcp/marketplace`,
        });
        if (result.error) return { error: result.error };
        return { data: result.data as MarketplaceResponse };
      },
      providesTags: ["MarketplaceServers"],
    }),

    installServer: builder.mutation<InstallResponse, InstallRequest>({
      queryFn: async (body, api, _opts, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort;
        const result = await baseQuery({
          url: `http://127.0.0.1:${port}/v1/mcp/marketplace/install`,
          method: "POST",
          body,
        });
        if (result.error) return { error: result.error };
        return { data: result.data as InstallResponse };
      },
      invalidatesTags: ["InstalledServers"],
    }),

    getInstalledServers: builder.query<InstalledResponse, undefined>({
      queryFn: async (_arg, api, _opts, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort;
        const result = await baseQuery({
          url: `http://127.0.0.1:${port}/v1/mcp/marketplace/installed`,
        });
        if (result.error) return { error: result.error };
        return { data: result.data as InstalledResponse };
      },
      providesTags: ["InstalledServers"],
    }),
  }),
});

export const {
  useGetMarketplaceQuery,
  useInstallServerMutation,
  useGetInstalledServersQuery,
} = mcpMarketplaceApi;
