import React, { useCallback, useMemo, useRef, useState } from "react";
import { Flex, Skeleton, Spinner, Text, TextField } from "@radix-ui/themes";
import { MagnifyingGlassIcon, ChevronDownIcon, ChevronUpIcon } from "@radix-ui/react-icons";
import { useAppDispatch, useAppSelector, useLoadMoreHistory } from "../../../../hooks";
import {
  buildHistoryTree,
  ChatHistoryItem,
  deleteChatById,
  HistoryTreeNode,
  updateChatTitleById,
} from "../../../History/historySlice";
import { restoreChat } from "../../../Chat/Thread";
import { push } from "../../../Pages/pagesSlice";
import { RecentItem, getDateGroup } from "./RecentItem";
import type { DashboardBreakpoint } from "../../types";
import styles from "./RecentSection.module.css";

type RecentSectionProps = {
  breakpoint: DashboardBreakpoint;
  expanded: boolean;
  onToggleExpand: () => void;
};

function treeMatchesQuery(node: HistoryTreeNode, query: string): boolean {
  if (
    node.title.toLowerCase().includes(query) ||
    (node.mode?.toLowerCase().includes(query) ?? false)
  ) {
    return true;
  }
  return node.children.some((child) => treeMatchesQuery(child, query));
}

export const RecentSection: React.FC<RecentSectionProps> = ({
  breakpoint,
  expanded,
  onToggleExpand,
}) => {
  const dispatch = useAppDispatch();
  const isInitialLoading = useAppSelector((state) => state.history.isLoading);
  const history = useAppSelector((state) => state.history.chats, {
    devModeChecks: { stabilityCheck: "never" },
  });

  const [searchQuery, setSearchQuery] = useState("");

  const {
    loadMore: loadMoreAsync,
    hasMore,
    isLoading: isLoadingMore,
    error: loadMoreError,
    retry: retryLoadMore,
  } = useLoadMoreHistory();

  const tree = useMemo(() => buildHistoryTree(history), [history]);

  const filteredTree = useMemo(() => {
    if (!searchQuery.trim()) return tree;
    const q = searchQuery.toLowerCase();
    return tree.filter((n) => treeMatchesQuery(n, q));
  }, [tree, searchQuery]);

  const handleItemClick = useCallback(
    (node: HistoryTreeNode) => {
      const item = history[node.id] as ChatHistoryItem | undefined;
      dispatch(restoreChat(item ?? (node as unknown as ChatHistoryItem)));
      dispatch(push({ name: "chat" }));
    },
    [dispatch, history],
  );

  const handleDotClick = useCallback(
    (chatId: string) => {
      const item = history[chatId] as ChatHistoryItem | undefined;
      if (item) {
        dispatch(restoreChat(item));
        dispatch(push({ name: "chat" }));
      }
    },
    [dispatch, history],
  );

  const handleDelete = useCallback(
    (id: string) => {
      dispatch(deleteChatById(id));
    },
    [dispatch],
  );

  const handleRename = useCallback(
    (id: string, newTitle: string) => {
      dispatch(updateChatTitleById({ chatId: id, newTitle }));
    },
    [dispatch],
  );

  const GROUP_ORDER = ["Today", "Yesterday", "Last 7 days", "Older"];

  const grouped = useMemo(() => {
    if (!expanded) return null;
    const groups = new Map<string, HistoryTreeNode[]>();
    for (const label of GROUP_ORDER) {
      groups.set(label, []);
    }
    for (const node of filteredTree) {
      const group = getDateGroup(node.updatedAt);
      if (!groups.has(group)) groups.set(group, []);
      const arr = groups.get(group);
      if (arr) arr.push(node);
    }
    const result = new Map<string, HistoryTreeNode[]>();
    for (const [key, nodes] of groups) {
      if (nodes.length > 0) result.set(key, nodes);
    }
    return result;
  }, [expanded, filteredTree]);

  const listRef = useRef<HTMLDivElement>(null);

  const handleScroll = useCallback(() => {
    if (!hasMore || isLoadingMore) return;
    const el = listRef.current;
    if (!el) return;
    const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 100;
    if (nearBottom) {
      void loadMoreAsync();
    }
  }, [hasMore, isLoadingMore, loadMoreAsync]);

  return (
    <div className={styles.section}>
      <button
        type="button"
        className={styles.header}
        onClick={onToggleExpand}
      >
        <Text size="1" weight="bold" color="gray" className={styles.label}>
          RECENT
        </Text>
        <Flex align="center" gap="1">
          {!expanded && (
            <Text size="1" color="gray">
              {filteredTree.length} total
            </Text>
          )}
          {expanded ? (
            <ChevronUpIcon width={12} height={12} color="var(--gray-9)" />
          ) : (
            <ChevronDownIcon width={12} height={12} color="var(--gray-9)" />
          )}
        </Flex>
      </button>

      {expanded && (
        <div className={styles.controls}>
          <TextField.Root
            size="1"
            placeholder="Search..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          >
            <TextField.Slot>
              <MagnifyingGlassIcon width={12} height={12} />
            </TextField.Slot>
          </TextField.Root>
        </div>
      )}

      <div
        ref={listRef}
        className={styles.list}
        onScroll={handleScroll}
      >
        {expanded && grouped ? (
          Array.from(grouped.entries()).map(([group, nodes]) => (
            <div key={group}>
              <Text
                size="1"
                color="gray"
                className={styles.groupLabel}
              >
                {group}
              </Text>
              {nodes.map((node) => (
                <RecentItem
                  key={node.id}
                  node={node}
                  breakpoint={breakpoint}
                  onClick={() => handleItemClick(node)}
                  onDotClick={handleDotClick}
                  onDelete={handleDelete}
                  onRename={handleRename}
                />
              ))}
            </div>
          ))
        ) : (
          filteredTree.map((node) => (
            <RecentItem
              key={node.id}
              node={node}
              breakpoint={breakpoint}
              onClick={() => handleItemClick(node)}
              onDotClick={handleDotClick}
              onDelete={handleDelete}
              onRename={handleRename}
            />
          ))
        )}
        {isInitialLoading && filteredTree.length === 0 && (
          <Flex direction="column" gap="1" p="1">
            {Array.from({ length: 8 }, (_, i) => (
              <Flex key={i} align="center" gap="2" py="1" px="2">
                <Skeleton><div style={{ width: 8, height: 8, borderRadius: "50%" }} /></Skeleton>
                <Skeleton><Text size="2" style={{ width: `${120 + (i % 3) * 40}px` }}>&nbsp;</Text></Skeleton>
                <div style={{ flex: 1 }} />
                <Skeleton><Text size="1" style={{ width: 40 }}>&nbsp;</Text></Skeleton>
              </Flex>
            ))}
          </Flex>
        )}
        {isLoadingMore && (
          <Flex justify="center" py="2">
            <Spinner size="2" />
          </Flex>
        )}
        {loadMoreError && (
          <Flex justify="center" py="2">
            <Text
              size="1"
              color="red"
              style={{ cursor: "pointer" }}
              onClick={retryLoadMore}
            >
              Load failed — click to retry
            </Text>
          </Flex>
        )}
        {!isInitialLoading && filteredTree.length === 0 && (
          <Text size="2" color="gray" style={{ padding: "var(--space-4)", textAlign: "center" }}>
            {searchQuery ? "No matching chats" : "No chats yet"}
          </Text>
        )}
      </div>
    </div>
  );
};
