import React, { useCallback, useMemo } from "react";
import { Badge, Flex, Skeleton, Text } from "@radix-ui/themes";
import { ChevronRightIcon, PlusIcon } from "@radix-ui/react-icons";
import { Virtuoso } from "react-virtuoso";
import { useAppDispatch } from "../../../../hooks";
import { push } from "../../../Pages/pagesSlice";
import { useListTasksQuery, useCreateTaskMutation } from "../../../../services/refact/tasks";
import { StatusDot } from "../../../../components/StatusDot";
import { getTaskStatusDotState } from "../../../../utils/sessionStatus";
import type { TaskMeta } from "../../../../services/refact/tasks";
import type { DashboardBreakpoint } from "../../types";
import styles from "./TasksSection.module.css";

type TasksSectionProps = {
  breakpoint: DashboardBreakpoint;
};

function formatTaskTime(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffHr = Math.floor(diffMs / 3_600_000);
  const diffDay = Math.floor(diffMs / 86_400_000);

  if (diffHr < 1) return "just now";
  if (diffHr < 24) return `${diffHr}h ago`;
  if (diffDay < 7) return `${diffDay}d ago`;
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function getStatusColor(status: string): "blue" | "purple" | "amber" | "green" | "red" | "gray" {
  switch (status) {
    case "active": return "blue";
    case "planning": return "purple";
    case "paused": return "amber";
    case "completed": return "green";
    case "abandoned": return "red";
    default: return "gray";
  }
}

export const TasksSection: React.FC<TasksSectionProps> = ({
  breakpoint,
}) => {
  const dispatch = useAppDispatch();
  const { data: tasks, isLoading, isError } = useListTasksQuery(undefined);
  const [createTask, { isLoading: isCreatingTask }] = useCreateTaskMutation();
  const sortedTasks = useMemo(() => {
    if (!tasks) return [];
    const priority = new Map([
      ["active", 0], ["planning", 1], ["paused", 2], ["completed", 3], ["abandoned", 4],
    ]);
    return [...tasks].sort((a, b) => {
      const pa = priority.get(a.status) ?? 999;
      const pb = priority.get(b.status) ?? 999;
      if (pa !== pb) return pa - pb;
      return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime();
    });
  }, [tasks]);

  const handleTaskClick = useCallback(
    (task: TaskMeta) => {
      dispatch(push({ name: "task workspace", taskId: task.id }));
    },
    [dispatch],
  );

  const handleNewTask = useCallback(() => {
    void createTask({ name: "New Task" })
      .unwrap()
      .then((task) => {
        dispatch(push({ name: "task workspace", taskId: task.id }));
      })
      .catch(() => {
        // Task creation failed
      });
  }, [createTask, dispatch]);

  if (isLoading) {
    return (
      <div className={styles.section}>
        <div className={styles.header}>
          <Text size="1" weight="bold" color="gray" className={styles.label}>TASKS</Text>
        </div>
        <Flex direction="column" gap="1">
          {Array.from({ length: 3 }, (_, i) => (
            <Skeleton key={i} height="28px" />
          ))}
        </Flex>
      </div>
    );
  }

  if (isError) {
    return (
      <div className={styles.section}>
        <div className={styles.header}>
          <Text size="1" weight="bold" color="gray" className={styles.label}>TASKS</Text>
        </div>
        <Text size="1" color="red">Failed to load tasks</Text>
      </div>
    );
  }

  if (sortedTasks.length === 0) {
    return (
      <div className={styles.section}>
        <div className={styles.header}>
          <Text size="1" weight="bold" color="gray" className={styles.label}>TASKS</Text>
          <button
            type="button"
            className={styles.newTaskButton}
            onClick={handleNewTask}
            disabled={isCreatingTask}
          >
            <PlusIcon width={12} height={12} />
            <Text size="1">New Task</Text>
          </button>
        </div>
        <Text size="1" color="gray" style={{ padding: "var(--space-2)", textAlign: "center" }}>
          No tasks yet
        </Text>
      </div>
    );
  }

  const useVirtualization = sortedTasks.length > 20;

  const renderTaskItem = (task: TaskMeta) => {
    const progress = task.cards_total > 0
      ? Math.min(100, Math.max(0, Math.round((task.cards_done / task.cards_total) * 100)))
      : 0;
    return (
      <div
        key={task.id}
        role="button"
        tabIndex={0}
        className={styles.taskItem}
        onClick={() => handleTaskClick(task)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            handleTaskClick(task);
          }
        }}
      >
        <div className={styles.taskLeft}>
          <StatusDot state={getTaskStatusDotState(task)} size="small" />
          <Text size="2" truncate className={styles.taskName}>
            {task.name}
          </Text>
        </div>
        <div className={styles.taskRight}>
          {task.cards_total > 0 && (
            <Flex align="center" gap="1">
              <div className={styles.progressBar}>
                <div
                  className={styles.progressFill}
                  style={{ width: `${progress}%` }}
                />
              </div>
              <Text size="1" color="gray">
                {task.cards_done}/{task.cards_total}
              </Text>
            </Flex>
          )}
          {breakpoint !== "narrow" && task.agents_active > 0 && (
            <Text size="1" color="gray">
              {task.agents_active} agent{task.agents_active !== 1 ? "s" : ""}
            </Text>
          )}
          {breakpoint !== "narrow" && task.cards_failed > 0 && (
            <Text size="1" color="red">
              {task.cards_failed} failed
            </Text>
          )}
          {breakpoint === "wide" && task.base_branch && (
            <Text size="1" color="gray" truncate style={{ maxWidth: 80 }}>
              {task.base_branch}
            </Text>
          )}
          <Badge size="1" variant="soft" color={getStatusColor(task.status)}>
            {task.status}
          </Badge>
          <Text size="1" color="gray" className={styles.taskTime}>
            {formatTaskTime(task.updated_at)}
          </Text>
          <ChevronRightIcon width={12} height={12} color="var(--gray-8)" className={styles.taskChevron} />
        </div>
      </div>
    );
  };

  return (
    <div className={styles.section}>
      <div className={styles.header}>
        <Flex align="center" gap="2">
          <Text size="1" weight="bold" color="gray" className={styles.label}>
            TASKS ({sortedTasks.length})
          </Text>
          <Text size="1" color="gray">
            {sortedTasks.filter((t) => t.status === "active" || t.status === "planning").length} active
          </Text>
        </Flex>
        <button
          type="button"
          className={styles.newTaskButton}
          onClick={handleNewTask}
          disabled={isCreatingTask}
        >
          <PlusIcon width={12} height={12} />
          <Text size="1">New Task</Text>
        </button>
      </div>
      <div className={styles.list}>
        {useVirtualization ? (
          <Virtuoso
            data={sortedTasks}
            overscan={100}
            className={styles.virtuosoList}
            itemContent={(_index, task) => renderTaskItem(task)}
          />
        ) : (
          sortedTasks.map((task) => renderTaskItem(task))
        )}
      </div>

    </div>
  );
};
