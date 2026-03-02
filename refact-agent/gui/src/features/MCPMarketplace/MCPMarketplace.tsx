import React, { useMemo, useState } from "react";
import {
  Badge,
  Box,
  Button,
  Callout,
  Flex,
  Heading,
  Text,
  TextField,
} from "@radix-ui/themes";
import { ArrowLeftIcon, InfoCircledIcon, MagnifyingGlassIcon } from "@radix-ui/react-icons";
import { ScrollArea } from "../../components/ScrollArea";
import { PageWrapper } from "../../components/PageWrapper";
import {
  useGetMarketplaceQuery,
  useGetInstalledServersQuery,
  useInstallServerMutation,
} from "../../services/refact/mcpMarketplace";
import type { MCPServer } from "../../services/refact/mcpMarketplace";
import { ServerCard } from "./ServerCard";
import { ServerDetail } from "./ServerDetail";
import styles from "./MCPMarketplace.module.css";
import type { Config } from "../Config/configSlice";
import { Spinner } from "../../components/Spinner";

type MCPMarketplaceProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
  backFromMarketplace: () => void;
};

export const MCPMarketplace: React.FC<MCPMarketplaceProps> = ({
  host,
  backFromMarketplace,
}) => {
  const [search, setSearch] = useState("");
  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const [selectedServer, setSelectedServer] = useState<MCPServer | null>(null);
  const [installingId, setInstallingId] = useState<string | null>(null);

  const { data: marketplaceData, isLoading, error } = useGetMarketplaceQuery(undefined);
  const { data: installedData } = useGetInstalledServersQuery(undefined);
  const [installServer] = useInstallServerMutation();

  const installedIds = useMemo(
    () => new Set((installedData?.installed ?? []).map((s) => s.id)),
    [installedData],
  );

  const allTags = useMemo(() => {
    const tagSet = new Set<string>();
    (marketplaceData?.servers ?? []).forEach((s) =>
      s.tags.forEach((t) => tagSet.add(t)),
    );
    return Array.from(tagSet).sort();
  }, [marketplaceData]);

  const filteredServers = useMemo(() => {
    const servers = marketplaceData?.servers ?? [];
    const q = search.toLowerCase();
    return servers.filter((s) => {
      const matchesSearch =
        !q ||
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        s.tags.some((t) => t.toLowerCase().includes(q));
      const matchesTag = !selectedTag || s.tags.includes(selectedTag);
      return matchesSearch && matchesTag;
    });
  }, [marketplaceData, search, selectedTag]);

  const handleInstall = async (server: MCPServer) => {
    setInstallingId(server.id);
    try {
      await installServer({ server_id: server.id });
    } finally {
      setInstallingId(null);
    }
  };

  const errorMessage =
    error && "data" in error
      ? String(error.data)
      : error
        ? "Failed to load marketplace"
        : null;

  if (selectedServer) {
    return (
      <PageWrapper host={host} style={{ padding: "var(--space-4)" }}>
        <ScrollArea scrollbars="vertical" fullHeight>
          <ServerDetail
            server={selectedServer}
            onBack={() => setSelectedServer(null)}
          />
        </ScrollArea>
      </PageWrapper>
    );
  }

  return (
    <PageWrapper host={host} style={{ padding: "var(--space-4)" }}>
      <ScrollArea scrollbars="vertical" fullHeight>
        <Flex direction="column" gap="4">
          <Flex align="center" gap="3">
            <Button variant="ghost" size="1" onClick={backFromMarketplace}>
              <ArrowLeftIcon />
              Back
            </Button>
            <Heading size="4">MCP Marketplace</Heading>
            {marketplaceData && (
              <Badge color="gray" variant="soft" className={styles.sourceBadge}>
                {marketplaceData.source}
              </Badge>
            )}
          </Flex>

          <Flex gap="2" align="center">
            <Box style={{ flex: 1 }}>
              <TextField.Root
                size="2"
                placeholder="Search servers…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              >
                <TextField.Slot>
                  <MagnifyingGlassIcon />
                </TextField.Slot>
              </TextField.Root>
            </Box>
          </Flex>

          {allTags.length > 0 && (
            <Flex gap="2" wrap="wrap">
              <Badge
                color={selectedTag === null ? "blue" : "gray"}
                variant={selectedTag === null ? "solid" : "soft"}
                style={{ cursor: "pointer" }}
                onClick={() => setSelectedTag(null)}
              >
                All
              </Badge>
              {allTags.map((tag) => (
                <Badge
                  key={tag}
                  color={selectedTag === tag ? "blue" : "gray"}
                  variant={selectedTag === tag ? "solid" : "soft"}
                  style={{ cursor: "pointer" }}
                  onClick={() =>
                    setSelectedTag(selectedTag === tag ? null : tag)
                  }
                >
                  {tag}
                </Badge>
              ))}
            </Flex>
          )}

          {errorMessage && (
            <Callout.Root color="red" size="1">
              <Callout.Icon>
                <InfoCircledIcon />
              </Callout.Icon>
              <Callout.Text>{errorMessage}</Callout.Text>
            </Callout.Root>
          )}

          {isLoading && <Spinner spinning />}

          {!isLoading && !errorMessage && filteredServers.length === 0 && (
            <Text size="2" color="gray" align="center">
              No servers found
            </Text>
          )}

          {!isLoading && filteredServers.length > 0 && (
            <div className={styles.serverGrid}>
              {filteredServers.map((server) => (
                <ServerCard
                  key={server.id}
                  server={server}
                  isInstalled={installedIds.has(server.id)}
                  isInstalling={installingId === server.id}
                  onInstall={(s) => void handleInstall(s)}
                  onViewDetail={(s) => setSelectedServer(s)}
                />
              ))}
            </div>
          )}
        </Flex>
      </ScrollArea>
    </PageWrapper>
  );
};
