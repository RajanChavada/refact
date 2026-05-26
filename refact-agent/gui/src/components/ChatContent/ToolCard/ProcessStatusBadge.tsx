import React from "react";
import { Badge } from "@radix-ui/themes";
import classNames from "classnames";

import type { ExecProcessStatus } from "../../../services/refact/types";
import styles from "./ExecToolCard.module.css";

type ProcessStatusBadgeProps = {
  status: ExecProcessStatus;
};

const STATUS_CLASS: Record<ExecProcessStatus, string> = {
  starting: styles.statusStarting,
  running: styles.statusRunning,
  exited: styles.statusExited,
  failed: styles.statusFailed,
  killed: styles.statusKilled,
  timed_out: styles.statusTimedOut,
};

const statusLabel = (status: ExecProcessStatus): string => {
  return status === "timed_out" ? "timed out" : status;
};

export const ProcessStatusBadge: React.FC<ProcessStatusBadgeProps> = ({
  status,
}) => {
  return (
    <Badge
      size="1"
      variant="soft"
      className={classNames(styles.statusBadge, STATUS_CLASS[status])}
      data-testid={`exec-status-${status}`}
    >
      {statusLabel(status)}
    </Badge>
  );
};

export default ProcessStatusBadge;
