import React from "react";
import { Button, Flex, Popover, Separator, Text } from "@radix-ui/themes";
import type { WorktreeMeta, WorktreeRecordView } from "../../services/refact";
import { WorktreeStatusBadge } from "./WorktreeStatusBadge";
import styles from "./Worktrees.module.css";

type WorktreeMenuProps = {
  currentWorktree: WorktreeMeta | null;
  currentRecord?: WorktreeRecordView | null;
  records: WorktreeRecordView[];
  isLoading: boolean;
  feedback?: string | null;
  canCopyPath: boolean;
  onCreate: () => void;
  onSelect: (record: WorktreeRecordView) => void;
  onDetach: () => void;
  onOpenInNewWindow: () => void;
  onCopyPath: () => void;
};

function compactPath(path: string): string {
  const normalized = path.replace(/[\\/]+$/, "");
  const parts = normalized.split(/[\\/]/).filter(Boolean);
  if (parts.length <= 2) return normalized || path;
  return parts.slice(-2).join("/");
}

function displayName(worktree: WorktreeMeta): string {
  const branch = worktree.branch?.trim();
  return branch !== undefined && branch.length > 0
    ? branch
    : compactPath(worktree.root);
}

function referencesLabel(record: WorktreeRecordView): string {
  if (record.reference_count === 0) return "unused";
  if (record.reference_count === 1) return "1 reference";
  return `${record.reference_count} references`;
}

export const WorktreeMenu: React.FC<WorktreeMenuProps> = ({
  currentWorktree,
  currentRecord,
  records,
  isLoading,
  feedback,
  canCopyPath,
  onCreate,
  onSelect,
  onDetach,
  onOpenInNewWindow,
  onCopyPath,
}) => {
  return (
    <Popover.Content
      className={styles.content}
      side="top"
      align="start"
      sideOffset={8}
    >
      <div className={styles.menu}>
        <Flex direction="column" gap="1" className={styles.sectionHeader}>
          <Text size="2" weight="bold">
            Worktrees
          </Text>
          <Text size="1" color="gray">
            Create or share an isolated branch workspace for this chat.
          </Text>
        </Flex>

        {feedback && (
          <Text size="1" color="gray" className={styles.feedback}>
            {feedback}
          </Text>
        )}

        <div className={styles.actions}>
          <Button type="button" size="1" variant="soft" onClick={onCreate}>
            Create worktree for this chat
          </Button>
          <Button
            type="button"
            size="1"
            variant="soft"
            color="gray"
            onClick={onDetach}
            disabled={!currentWorktree}
          >
            Detach / use main workspace
          </Button>
          <Button
            type="button"
            size="1"
            variant="soft"
            color="gray"
            onClick={onOpenInNewWindow}
            disabled={!currentWorktree}
          >
            Open in new window
          </Button>
          <Button
            type="button"
            size="1"
            variant="soft"
            color="gray"
            onClick={onCopyPath}
            disabled={!canCopyPath}
          >
            Copy path
          </Button>
        </div>

        <Separator size="4" />

        <div className={styles.section}>
          <Text size="1" color="gray" className={styles.sectionHeader}>
            Existing worktrees
          </Text>
          <div className={styles.list}>
            {isLoading && (
              <Text size="1" color="gray" className={styles.sectionHeader}>
                Loading...
              </Text>
            )}
            {!isLoading && records.length === 0 && (
              <Text size="1" color="gray" className={styles.sectionHeader}>
                No worktrees yet.
              </Text>
            )}
            {records.map((record) => {
              const selected = currentWorktree?.id === record.meta.id;
              const title = displayName(record.meta);
              const usedBy = record.referencing_chat_ids?.length
                ? record.referencing_chat_ids.join(", ")
                : record.references
                    .map((reference) => reference.chat_id)
                    .filter((chatId): chatId is string => Boolean(chatId))
                    .join(", ");
              return (
                <button
                  key={record.meta.id}
                  type="button"
                  className={`${styles.item} ${
                    selected ? styles.itemSelected : ""
                  }`}
                  onClick={() => onSelect(record)}
                  aria-label={`Select worktree ${title}`}
                >
                  <Flex direction="column" gap="1" className={styles.itemTitle}>
                    <Flex align="center" gap="2" wrap="wrap">
                      <Text size="1" weight="medium">
                        {title}
                      </Text>
                      <WorktreeStatusBadge
                        worktree={record.meta}
                        record={record}
                      />
                    </Flex>
                    <Text size="1" color="gray" className={styles.path}>
                      {record.meta.root}
                    </Text>
                    <Text size="1" color="gray">
                      {referencesLabel(record)}
                      {usedBy ? ` · used by ${usedBy}` : ""}
                    </Text>
                  </Flex>
                </button>
              );
            })}
          </div>
        </div>

        <Separator size="4" />

        <div className={styles.actions}>
          <Button type="button" size="1" variant="soft" color="gray" disabled>
            Diff
          </Button>
          <Button type="button" size="1" variant="soft" color="gray" disabled>
            Merge
          </Button>
          <Button type="button" size="1" variant="soft" color="gray" disabled>
            Delete
          </Button>
        </div>

        {currentRecord?.reference_count && currentRecord.reference_count > 1 ? (
          <Text size="1" color="gray" className={styles.feedback}>
            This worktree is shared. Selecting it here is allowed.
          </Text>
        ) : null}
      </div>
    </Popover.Content>
  );
};

WorktreeMenu.displayName = "WorktreeMenu";
