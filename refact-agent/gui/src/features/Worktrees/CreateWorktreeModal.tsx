import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  Button,
  Dialog,
  Flex,
  Select,
  Text,
  TextField,
} from "@radix-ui/themes";
import styles from "./Worktrees.module.css";

export type CreateWorktreeValues = {
  branch?: string;
  baseBranch?: string;
};

type CreateWorktreeModalProps = {
  open: boolean;
  defaultBranch: string;
  defaultBaseBranch: string;
  baseBranchOptions: string[];
  isCreating: boolean;
  error?: string | null;
  onOpenChange: (open: boolean) => void;
  onCreate: (values: CreateWorktreeValues) => Promise<void>;
};

export const CreateWorktreeModal: React.FC<CreateWorktreeModalProps> = ({
  open,
  defaultBranch,
  defaultBaseBranch,
  baseBranchOptions,
  isCreating,
  error,
  onOpenChange,
  onCreate,
}) => {
  const [branchName, setBranchName] = useState(defaultBranch);
  const [baseBranch, setBaseBranch] = useState(defaultBaseBranch);

  useEffect(() => {
    if (open) {
      setBranchName(defaultBranch);
      setBaseBranch(defaultBaseBranch);
    }
  }, [open, defaultBranch, defaultBaseBranch]);

  const normalizedBaseOptions = useMemo(() => {
    const seen = new Set<string>();
    return baseBranchOptions
      .concat(defaultBaseBranch)
      .map((branch) => branch.trim())
      .filter((branch) => branch.length > 0)
      .filter((branch) => {
        if (seen.has(branch)) return false;
        seen.add(branch);
        return true;
      });
  }, [baseBranchOptions, defaultBaseBranch]);

  const handleCreate = useCallback(async () => {
    await onCreate({
      branch: branchName.trim() || undefined,
      baseBranch: baseBranch.trim() || undefined,
    });
  }, [baseBranch, branchName, onCreate]);

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Content maxWidth="420px">
        <Dialog.Title>Create worktree</Dialog.Title>
        <Dialog.Description size="2" color="gray">
          Create a new git worktree and attach it to this chat.
        </Dialog.Description>

        <div className={styles.modalFields}>
          <label className={styles.field} htmlFor="worktree-branch-name">
            <Text size="2" weight="medium">
              Branch name
            </Text>
            <TextField.Root
              id="worktree-branch-name"
              value={branchName}
              placeholder={defaultBranch}
              onChange={(event) => setBranchName(event.target.value)}
              disabled={isCreating}
            />
          </label>

          <div className={styles.field}>
            <Text size="2" weight="medium">
              Base branch
            </Text>
            {normalizedBaseOptions.length > 0 ? (
              <Select.Root
                value={baseBranch || normalizedBaseOptions[0]}
                onValueChange={setBaseBranch}
                disabled={isCreating}
              >
                <Select.Trigger aria-label="Base branch" />
                <Select.Content>
                  {normalizedBaseOptions.map((branch) => (
                    <Select.Item key={branch} value={branch}>
                      {branch}
                    </Select.Item>
                  ))}
                </Select.Content>
              </Select.Root>
            ) : (
              <TextField.Root
                value={baseBranch}
                placeholder="main"
                onChange={(event) => setBaseBranch(event.target.value)}
                disabled={isCreating}
              />
            )}
          </div>

          {error && (
            <Text size="1" color="red">
              {error}
            </Text>
          )}
        </div>

        <Flex className={styles.modalActions}>
          <Dialog.Close>
            <Button
              type="button"
              variant="soft"
              color="gray"
              disabled={isCreating}
            >
              Cancel
            </Button>
          </Dialog.Close>
          <Button
            type="button"
            onClick={() => void handleCreate()}
            disabled={isCreating}
          >
            {isCreating ? "Creating..." : "Create"}
          </Button>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
};

CreateWorktreeModal.displayName = "CreateWorktreeModal";
