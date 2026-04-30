import React from "react";
import { Badge } from "@radix-ui/themes";
import type { WorktreeMeta, WorktreeRecordView } from "../../services/refact";

type WorktreeStatusBadgeProps = {
  worktree?: WorktreeMeta | null;
  record?: WorktreeRecordView | null;
};

export const WorktreeStatusBadge: React.FC<WorktreeStatusBadgeProps> = ({
  worktree,
  record,
}) => {
  const status = record?.status ?? worktree?.status ?? null;
  const lifecycle = record?.meta.lifecycle_state ?? worktree?.lifecycle_state;

  if (
    lifecycle === "deleted" ||
    worktree?.deleted === true ||
    status?.deleted === true
  ) {
    return (
      <Badge size="1" color="red" variant="soft">
        deleted
      </Badge>
    );
  }

  if (lifecycle === "missing" || status?.path_exists === false) {
    return (
      <Badge size="1" color="red" variant="soft">
        missing
      </Badge>
    );
  }

  if (lifecycle === "conflicted" || status?.conflicted === true) {
    return (
      <Badge size="1" color="amber" variant="soft">
        conflicted
      </Badge>
    );
  }

  if (
    lifecycle === "stale" ||
    worktree?.stale === true ||
    status?.stale === true
  ) {
    return (
      <Badge size="1" color="amber" variant="soft">
        stale
      </Badge>
    );
  }

  if (status?.dirty === true) {
    return (
      <Badge size="1" color="amber" variant="soft">
        dirty
      </Badge>
    );
  }

  return (
    <Badge size="1" color="green" variant="soft">
      worktree
    </Badge>
  );
};

WorktreeStatusBadge.displayName = "WorktreeStatusBadge";
