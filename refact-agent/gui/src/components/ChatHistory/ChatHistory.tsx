import { memo, useState, useCallback } from "react";
import { Flex, Box, Text, IconButton } from "@radix-ui/themes";
import { ChevronDownIcon, ChevronRightIcon } from "@radix-ui/react-icons";
import { ScrollArea } from "../ScrollArea";
import { HistoryItem } from "./HistoryItem";
import {
  ChatHistoryItem,
  getHistory,
  getHistoryTree,
  HistoryTreeNode,
  type HistoryState,
} from "../../features/History/historySlice";

export type ChatHistoryProps = {
  history: HistoryState;
  onHistoryItemClick: (id: ChatHistoryItem) => void;
  onDeleteHistoryItem: (id: string) => void;
  onOpenChatInTab?: (id: string) => void;
  currentChatId?: string;
  treeView?: boolean;
};

type TreeNodeProps = {
  node: HistoryTreeNode;
  depth: number;
  onHistoryItemClick: (id: ChatHistoryItem) => void;
  onDeleteHistoryItem: (id: string) => void;
  onOpenChatInTab?: (id: string) => void;
  currentChatId?: string;
  expandedIds: Set<string>;
  onToggleExpand: (id: string) => void;
};

const TreeNode = memo(
  ({
    node,
    depth,
    onHistoryItemClick,
    onDeleteHistoryItem,
    onOpenChatInTab,
    currentChatId,
    expandedIds,
    onToggleExpand,
  }: TreeNodeProps) => {
    const hasChildren = node.children.length > 0;
    const isExpanded = expandedIds.has(node.id);
    const isTask = !!node.task_id;

    return (
      <Box style={{ width: "100%" }}>
        <Flex align="center" gap="1" style={{ paddingLeft: depth * 16 }}>
          {hasChildren ? (
            <IconButton
              size="1"
              variant="ghost"
              onClick={() => onToggleExpand(node.id)}
              style={{ minWidth: 20, minHeight: 20 }}
            >
              {isExpanded ? (
                <ChevronDownIcon width={12} height={12} />
              ) : (
                <ChevronRightIcon width={12} height={12} />
              )}
            </IconButton>
          ) : (
            <Box style={{ width: 20 }} />
          )}
          <Box style={{ flex: 1 }}>
            <HistoryItem
              onClick={() => onHistoryItemClick(node)}
              onOpenInTab={onOpenChatInTab}
              onDelete={onDeleteHistoryItem}
              historyItem={node}
              disabled={node.id === currentChatId}
              badge={
                isTask
                  ? node.task_role === "planner"
                    ? "Planner"
                    : node.task_role === "agents"
                      ? "Agent"
                      : undefined
                  : undefined
              }
            />
          </Box>
        </Flex>
        {hasChildren && isExpanded && (
          <Box>
            {node.children.map((child) => (
              <TreeNode
                key={child.id}
                node={child}
                depth={depth + 1}
                onHistoryItemClick={onHistoryItemClick}
                onDeleteHistoryItem={onDeleteHistoryItem}
                onOpenChatInTab={onOpenChatInTab}
                currentChatId={currentChatId}
                expandedIds={expandedIds}
                onToggleExpand={onToggleExpand}
              />
            ))}
          </Box>
        )}
      </Box>
    );
  },
);

TreeNode.displayName = "TreeNode";

export const ChatHistory = memo(
  ({
    history,
    onHistoryItemClick,
    onDeleteHistoryItem,
    onOpenChatInTab,
    currentChatId,
    treeView = false,
  }: ChatHistoryProps) => {
    const sortedHistory = getHistory({ history });
    const historyTree = getHistoryTree({ history });
    const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());

    const handleToggleExpand = useCallback((id: string) => {
      setExpandedIds((prev) => {
        const next = new Set(prev);
        if (next.has(id)) {
          next.delete(id);
        } else {
          next.add(id);
        }
        return next;
      });
    }, []);

    const hasTaskChats = sortedHistory.some((item) => !!item.task_id);
    const showTree = treeView || hasTaskChats;

    return (
      <Box
        style={{
          overflow: "hidden",
        }}
        pb="2"
        flexGrow="1"
      >
        <ScrollArea scrollbars="vertical">
          <Flex
            justify="center"
            align={sortedHistory.length > 0 ? "center" : "start"}
            pl="2"
            pr="2"
            gap="2"
            direction="column"
          >
            {sortedHistory.length !== 0 ? (
              showTree ? (
                historyTree.map((node) => (
                  <TreeNode
                    key={node.id}
                    node={node}
                    depth={0}
                    onHistoryItemClick={onHistoryItemClick}
                    onDeleteHistoryItem={onDeleteHistoryItem}
                    onOpenChatInTab={onOpenChatInTab}
                    currentChatId={currentChatId}
                    expandedIds={expandedIds}
                    onToggleExpand={handleToggleExpand}
                  />
                ))
              ) : (
                sortedHistory.map((item) => (
                  <HistoryItem
                    onClick={() => onHistoryItemClick(item)}
                    onOpenInTab={onOpenChatInTab}
                    onDelete={onDeleteHistoryItem}
                    key={item.id}
                    historyItem={item}
                    disabled={item.id === currentChatId}
                  />
                ))
              )
            ) : (
              <Text as="p" size="2" mt="2">
                Your chat history is currently empty. Click &quot;New Chat&quot;
                to start a conversation.
              </Text>
            )}
          </Flex>
        </ScrollArea>
      </Box>
    );
  },
);

ChatHistory.displayName = "ChatHistory";
