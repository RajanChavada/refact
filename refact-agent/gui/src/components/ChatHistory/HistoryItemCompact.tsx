import React, { useState, useCallback } from "react";
import {
  Text,
  IconButton,
  TextField,
  Badge,
  HoverCard,
} from "@radix-ui/themes";
import {
  ChatBubbleIcon,
  Pencil1Icon,
  Cross1Icon,
  CheckIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  TokensIcon,
} from "@radix-ui/react-icons";
import { StatusDot, StatusDotState } from "../StatusDot";
import type { ChatHistoryItem } from "../../features/History/historySlice";
import {
  useChatSessionStates,
  SessionState,
} from "../../hooks/useStreamingChatIds";
import styles from "./HistoryItemCompact.module.css";

export interface HistoryItemCompactProps {
  historyItem: ChatHistoryItem;
  onClick: () => void;
  onDelete: (id: string) => void;
  onRename?: (id: string, newTitle: string) => void;
  disabled: boolean;
  badge?: string;
  childCount?: number;
  isExpanded?: boolean;
  onToggleExpand?: () => void;
  isChild?: boolean;
}

function getStatusDotState(
  sessionState: SessionState | null,
  isUnread: boolean,
): StatusDotState {
  if (sessionState === "generating" || sessionState === "executing_tools") {
    return "streaming";
  }
  if (sessionState === "paused" || sessionState === "waiting_ide") {
    return "paused";
  }
  if (sessionState === "error") {
    return "error";
  }
  if (isUnread) {
    return "unread";
  }
  return "idle";
}

function getStatusTooltip(
  sessionState: SessionState | null,
  isUnread: boolean,
): string {
  if (sessionState === "generating" || sessionState === "executing_tools") {
    return "Generating response...";
  }
  if (sessionState === "paused" || sessionState === "waiting_ide") {
    return "Waiting for confirmation";
  }
  if (sessionState === "error") {
    return "An error occurred";
  }
  if (isUnread) {
    return "Unread messages";
  }
  return "Idle";
}

function formatDateTime(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays === 0) {
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
  if (diffDays === 1) {
    return "Yesterday";
  }
  if (diffDays < 7) {
    return date.toLocaleDateString([], { weekday: "short" });
  }
  return date.toLocaleDateString([], { month: "short", day: "numeric" });
}

function formatCoins(coins: number): string {
  if (coins >= 1000000) {
    return `${(coins / 1000000).toFixed(1)}M`;
  }
  if (coins >= 1000) {
    return `${(coins / 1000).toFixed(1)}K`;
  }
  if (coins >= 1) {
    return coins.toFixed(0);
  }
  return coins.toFixed(2);
}

interface TooltipButtonProps {
  onClick: (e: React.MouseEvent) => void;
  tooltip: string;
  children: React.ReactNode;
  className?: string;
}

const TooltipButton: React.FC<TooltipButtonProps> = ({
  onClick,
  tooltip,
  children,
  className,
}) => (
  <HoverCard.Root openDelay={200} closeDelay={100}>
    <HoverCard.Trigger>
      <IconButton
        size="1"
        variant="ghost"
        onClick={onClick}
        className={className}
        aria-label={tooltip}
      >
        {children}
      </IconButton>
    </HoverCard.Trigger>
    <HoverCard.Content size="1" side="top" align="center">
      <Text as="p" size="1">
        {tooltip}
      </Text>
    </HoverCard.Content>
  </HoverCard.Root>
);

export const HistoryItemCompact: React.FC<HistoryItemCompactProps> = ({
  historyItem,
  onClick,
  onDelete,
  onRename,
  disabled,
  badge,
  childCount,
  isExpanded,
  onToggleExpand,
  isChild = false,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [editValue, setEditValue] = useState(historyItem.title);
  const chatSessionStates = useChatSessionStates();
  const sessionState = chatSessionStates[historyItem.id] as SessionState | null;
  const isUnread = historyItem.read === false;
  const statusState = getStatusDotState(sessionState, isUnread);
  const statusTooltip = getStatusTooltip(sessionState, isUnread);
  const dateTimeString = formatDateTime(historyItem.updatedAt);
  const messageCount = historyItem.message_count ?? historyItem.messages.length;
  const totalCoins = historyItem.total_coins;
  const hasChildren = childCount !== undefined && childCount > 0;

  const handleStartEdit = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setEditValue(historyItem.title);
      setIsEditing(true);
    },
    [historyItem.title],
  );

  const handleCancelEdit = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setIsEditing(false);
      setEditValue(historyItem.title);
    },
    [historyItem.title],
  );

  const handleConfirmEdit = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (editValue.trim() && onRename) {
        onRename(historyItem.id, editValue.trim());
      }
      setIsEditing(false);
    },
    [editValue, historyItem.id, onRename],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        if (editValue.trim() && onRename) {
          onRename(historyItem.id, editValue.trim());
        }
        setIsEditing(false);
      } else if (e.key === "Escape") {
        setIsEditing(false);
        setEditValue(historyItem.title);
      }
    },
    [editValue, historyItem.id, historyItem.title, onRename],
  );

  const handleDelete = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      onDelete(historyItem.id);
    },
    [historyItem.id, onDelete],
  );

  const handleClick = useCallback(() => {
    if (!isEditing && !disabled) {
      onClick();
    }
  }, [isEditing, disabled, onClick]);

  const handleRowKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.target !== e.currentTarget) return;
      if (disabled) return;
      if ((e.key === "Enter" || e.key === " ") && !isEditing) {
        e.preventDefault();
        onClick();
      }
    },
    [disabled, isEditing, onClick],
  );

  const handleToggleExpand = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      onToggleExpand?.();
    },
    [onToggleExpand],
  );

  const handleChevronKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        e.stopPropagation();
        onToggleExpand?.();
      }
    },
    [onToggleExpand],
  );

  const itemClasses = [
    styles.item,
    disabled ? styles.disabled : "",
    isChild ? styles.childItem : "",
  ]
    .filter(Boolean)
    .join(" ");

  const chevronTooltip = `${childCount} related ${
    childCount === 1 ? "chat" : "chats"
  }`;

  return (
    <div
      className={itemClasses}
      onClick={handleClick}
      role="button"
      tabIndex={disabled ? -1 : 0}
      onKeyDown={handleRowKeyDown}
    >
      <div className={styles.chevronArea}>
        {hasChildren && onToggleExpand && (
          <HoverCard.Root openDelay={200} closeDelay={100}>
            <HoverCard.Trigger>
              <div
                className={styles.expandChevron}
                onClick={handleToggleExpand}
                onKeyDown={handleChevronKeyDown}
                role="button"
                tabIndex={0}
                aria-label={chevronTooltip}
                aria-expanded={isExpanded}
              >
                {isExpanded ? (
                  <ChevronDownIcon width={14} height={14} />
                ) : (
                  <ChevronRightIcon width={14} height={14} />
                )}
              </div>
            </HoverCard.Trigger>
            <HoverCard.Content size="1" side="top" align="center">
              <Text as="p" size="1">
                {chevronTooltip}
              </Text>
            </HoverCard.Content>
          </HoverCard.Root>
        )}
      </div>

      <div className={styles.leftSection}>
        <StatusDot
          state={statusState}
          size="small"
          tooltipText={statusTooltip}
        />
        {badge && (
          <Badge size="1" color="gray" variant="soft" style={{ flexShrink: 0 }}>
            {badge}
          </Badge>
        )}
      </div>

      <div className={styles.titleSection}>
        {isEditing ? (
          <TextField.Root
            size="1"
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            onKeyDown={handleKeyDown}
            onClick={(e) => e.stopPropagation()}
            autoFocus
            className={styles.editInput}
          />
        ) : (
          <Text
            as="span"
            size="2"
            weight={isUnread ? "bold" : "regular"}
            className={styles.title}
          >
            {historyItem.title}
          </Text>
        )}
      </div>

      <div className={styles.stats}>
        <ChatBubbleIcon width={12} height={12} />
        <Text size="1" color="gray">
          {messageCount}
        </Text>
        {totalCoins !== undefined && totalCoins > 0 && (
          <>
            <TokensIcon width={12} height={12} />
            <Text size="1" color="gray">
              {formatCoins(totalCoins)}
            </Text>
          </>
        )}
      </div>

      <Text size="1" color="gray" className={styles.date}>
        {dateTimeString}
      </Text>

      <div className={styles.actions}>
        {isEditing ? (
          <>
            <TooltipButton onClick={handleConfirmEdit} tooltip="Save">
              <CheckIcon width={12} height={12} />
            </TooltipButton>
            <TooltipButton onClick={handleCancelEdit} tooltip="Cancel">
              <Cross1Icon width={10} height={10} />
            </TooltipButton>
          </>
        ) : (
          <>
            {onRename && (
              <TooltipButton
                onClick={handleStartEdit}
                tooltip="Rename"
                className={styles.actionButton}
              >
                <Pencil1Icon width={12} height={12} />
              </TooltipButton>
            )}
            <TooltipButton
              onClick={handleDelete}
              tooltip="Delete"
              className={styles.actionButton}
            >
              <Cross1Icon width={10} height={10} />
            </TooltipButton>
          </>
        )}
      </div>
    </div>
  );
};
