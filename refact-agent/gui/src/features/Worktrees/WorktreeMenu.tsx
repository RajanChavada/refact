import React, { useCallback, useState } from "react";
import {
  Button,
  Checkbox,
  Dialog,
  Flex,
  Popover,
  Separator,
  Text,
} from "@radix-ui/themes";
import {
  useDeleteWorktreeMutation,
  type MergeWorktreeResponse,
  type WorktreeMeta,
  type WorktreeRecordView,
} from "../../services/refact";
import { sendUserMessage } from "../../services/refact/chatCommands";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { selectApiKey, selectLspPort } from "../Config/configSlice";
import { selectChatId, setThreadWorktree } from "../Chat/Thread";
import { WorktreeStatusBadge } from "./WorktreeStatusBadge";
import { WorktreeDiffPanel } from "./WorktreeDiffPanel";
import { MergeWorktreeModal } from "./MergeWorktreeModal";
import { buildWorktreeConflictPrompt } from "./worktreeConflict";
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

function referenceCount(
  worktree: WorktreeMeta | null,
  record?: WorktreeRecordView | null,
): number {
  return record?.reference_count ?? worktree?.reference_count ?? 0;
}

function errorText(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "object" && error !== null && "data" in error) {
    const data = (error as { data: unknown }).data;
    if (typeof data === "string") return data;
    if (typeof data === "object" && data !== null && "detail" in data) {
      return String((data as { detail: unknown }).detail);
    }
  }
  if (typeof error === "object" && error !== null && "error" in error) {
    return String((error as { error: unknown }).error);
  }
  return String(error);
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
  const dispatch = useAppDispatch();
  const chatId = useAppSelector(selectChatId);
  const lspPort = useAppSelector(selectLspPort);
  const apiKey = useAppSelector(selectApiKey) ?? undefined;
  const [diffOpen, setDiffOpen] = useState(false);
  const [mergeOpen, setMergeOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleteBranch, setDeleteBranch] = useState(false);
  const [localFeedback, setLocalFeedback] = useState<string | null>(null);
  const [deleteWorktree, deleteState] = useDeleteWorktreeMutation();
  const sharedCount = referenceCount(currentWorktree, currentRecord);
  const worktreeAvailable = Boolean(currentWorktree);
  const hasFeedback =
    (feedback?.length ?? 0) > 0 || (localFeedback?.length ?? 0) > 0;

  const handleAskRefact = useCallback(
    async (files: string[], response: MergeWorktreeResponse) => {
      if (!currentWorktree || !chatId || !lspPort) {
        throw new Error("No active worktree chat is available.");
      }
      const prompt = buildWorktreeConflictPrompt({
        worktree: currentWorktree,
        record: currentRecord,
        response,
        files,
      });
      await sendUserMessage(chatId, prompt, lspPort, apiKey, true);
      setLocalFeedback("Conflict resolution request sent to Refact.");
    },
    [apiKey, chatId, currentRecord, currentWorktree, lspPort],
  );

  const handleDelete = useCallback(async () => {
    if (!currentWorktree) return;
    setLocalFeedback(null);
    try {
      await deleteWorktree({
        id: currentWorktree.id,
        source_workspace_root: currentWorktree.source_workspace_root,
        delete_branch: deleteBranch,
      }).unwrap();
      setDeleteOpen(false);
      setLocalFeedback("Worktree deleted.");
      if (chatId && currentWorktree.id) {
        dispatch(setThreadWorktree({ chatId, worktree: null }));
        onDetach();
      }
    } catch (error) {
      setLocalFeedback(`Delete failed: ${errorText(error)}`);
    }
  }, [
    chatId,
    currentWorktree,
    deleteBranch,
    deleteWorktree,
    dispatch,
    onDetach,
  ]);

  return (
    <>
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

          {hasFeedback && (
            <Flex direction="column" gap="1" className={styles.feedback}>
              {feedback && (
                <Text size="1" color="gray">
                  {feedback}
                </Text>
              )}
              {localFeedback && (
                <Text size="1" color="gray">
                  {localFeedback}
                </Text>
              )}
            </Flex>
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
                      .filter((value): value is string => Boolean(value))
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
                    <Flex
                      direction="column"
                      gap="1"
                      className={styles.itemTitle}
                    >
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
            <Button
              type="button"
              size="1"
              variant="soft"
              color="gray"
              onClick={() => setDiffOpen(true)}
              disabled={!worktreeAvailable}
            >
              View Diff
            </Button>
            <Button
              type="button"
              size="1"
              variant="soft"
              color="gray"
              onClick={() => setMergeOpen(true)}
              disabled={!worktreeAvailable}
            >
              Merge
            </Button>
            <Button
              type="button"
              size="1"
              variant="soft"
              color="red"
              onClick={() => setDeleteOpen(true)}
              disabled={!worktreeAvailable}
            >
              Discard/Delete
            </Button>
          </div>

          {sharedCount > 1 ? (
            <Text size="1" color="gray" className={styles.feedback}>
              This worktree is shared by {sharedCount} references. Delete and
              discard actions can affect other chats.
            </Text>
          ) : null}
        </div>
      </Popover.Content>

      <WorktreeDiffPanel
        open={diffOpen}
        worktreeId={currentWorktree?.id}
        worktree={currentWorktree}
        record={currentRecord}
        onOpenChange={setDiffOpen}
      />

      <MergeWorktreeModal
        open={mergeOpen}
        worktreeId={currentWorktree?.id}
        worktree={currentWorktree}
        record={currentRecord}
        onOpenChange={setMergeOpen}
        onAskRefact={handleAskRefact}
        onOpenWorktree={onOpenInNewWindow}
      />

      <Dialog.Root open={deleteOpen} onOpenChange={setDeleteOpen}>
        <Dialog.Content maxWidth="420px">
          <Dialog.Title>Delete worktree</Dialog.Title>
          <Dialog.Description size="2" color="gray">
            Delete or discard the selected worktree from disk.
          </Dialog.Description>

          <Flex direction="column" gap="3" mt="3">
            <div className={styles.dialogOverlayText}>
              <Text size="2" weight="medium">
                {currentWorktree ? displayName(currentWorktree) : "No worktree"}
              </Text>
              {currentWorktree && (
                <Text size="1" color="gray" className={styles.path}>
                  {currentWorktree.root}
                </Text>
              )}
            </div>

            {sharedCount > 1 && (
              <Text size="2" color="amber" className={styles.warningBox}>
                This worktree is shared by {sharedCount} references. Deleting it
                may affect other chats that use the same worktree.
              </Text>
            )}

            <Text as="label" size="2">
              <Flex align="center" gap="2">
                <Checkbox
                  checked={deleteBranch}
                  onCheckedChange={(checked) =>
                    setDeleteBranch(checked === true)
                  }
                  disabled={deleteState.isLoading}
                />
                Delete git branch too
              </Flex>
            </Text>

            {localFeedback && localFeedback.startsWith("Delete failed") && (
              <Text size="2" color="red" className={styles.warningBox}>
                {localFeedback}
              </Text>
            )}
          </Flex>

          <Flex className={styles.modalActions}>
            <Dialog.Close>
              <Button
                type="button"
                variant="soft"
                color="gray"
                disabled={deleteState.isLoading}
              >
                Cancel
              </Button>
            </Dialog.Close>
            <Button
              type="button"
              color="red"
              onClick={() => void handleDelete()}
              disabled={!currentWorktree || deleteState.isLoading}
            >
              {deleteState.isLoading ? "Deleting..." : "Delete worktree"}
            </Button>
          </Flex>
        </Dialog.Content>
      </Dialog.Root>
    </>
  );
};

WorktreeMenu.displayName = "WorktreeMenu";
