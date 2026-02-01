import React, { useCallback } from "react";
import * as Collapsible from "@radix-ui/react-collapsible";
import { Flex, Text, Box, Separator } from "@radix-ui/themes";
import { CheckboxIcon } from "@radix-ui/react-icons";
import classNames from "classnames";

import { useAppSelector, useAppDispatch } from "../../hooks";
import {
  selectChatId,
  selectCurrentTasks,
  selectHasTasks,
  selectTasksEverUsed,
  selectTaskProgress,
  selectTaskWidgetExpanded,
  selectIsStreaming,
  setTaskWidgetExpanded,
} from "../../features/Chat/Thread";
import type { TodoItem, TodoStatus } from "../../features/Chat/Thread/types";
import { Chevron } from "../Collapsible";
import { AnimatedText } from "../Text";
import { StatusDot, type StatusDotState } from "../StatusDot";
import styles from "./TaskProgressWidget.module.css";

function getStatusDotState(
  status: TodoStatus,
  _isStreaming: boolean,
): StatusDotState {
  switch (status) {
    case "in_progress":
      return "paused"; // Blue pulsing for in-progress tasks
    case "completed":
      return "completed"; // Green solid for completed
    case "failed":
      return "error"; // Red for failed
    case "pending":
    default:
      return "idle"; // Gray for pending
  }
}

const STATUS_TOOLTIPS: Record<TodoStatus, string> = {
  completed: "Completed",
  in_progress: "In progress",
  pending: "Pending",
  failed: "Failed",
};

type StatusIconProps = {
  status: TodoStatus;
  isStreaming?: boolean;
};

const StatusIcon: React.FC<StatusIconProps> = ({
  status,
  isStreaming = false,
}) => {
  const dotState = getStatusDotState(status, isStreaming);
  return (
    <StatusDot
      state={dotState}
      size="small"
      tooltipText={STATUS_TOOLTIPS[status]}
    />
  );
};

type TaskRowProps = {
  task: TodoItem;
  isStreaming: boolean;
};

const TaskRow: React.FC<TaskRowProps> = ({ task, isStreaming }) => {
  const isActive = task.status === "in_progress";

  return (
    <Flex
      align="center"
      gap="2"
      className={classNames(styles.taskRow, { [styles.active]: isActive })}
    >
      <StatusIcon status={task.status} isStreaming={isStreaming && isActive} />
      <Text size="2" style={{ flex: 1 }}>
        {task.content}
      </Text>
    </Flex>
  );
};

type ProgressBarProps = {
  done: number;
  total: number;
  animating?: boolean;
};

const ProgressBar: React.FC<ProgressBarProps> = ({
  done,
  total,
  animating = false,
}) => {
  const percent = total > 0 ? (done / total) * 100 : 0;

  return (
    <Box className={styles.progressBar}>
      <Box
        className={classNames(styles.progressFill, {
          [styles.animating]: animating,
        })}
        style={{ width: `${percent}%` }}
      />
    </Box>
  );
};

export const TaskProgressWidget: React.FC = () => {
  const dispatch = useAppDispatch();
  const chatId = useAppSelector(selectChatId);
  const hasTasks = useAppSelector(selectHasTasks);
  const everUsed = useAppSelector(selectTasksEverUsed);
  const tasks = useAppSelector(selectCurrentTasks);
  const isExpanded = useAppSelector(selectTaskWidgetExpanded);
  const isStreaming = useAppSelector(selectIsStreaming);
  const { done, total, activeTitle } = useAppSelector(selectTaskProgress);

  const handleOpenChange = useCallback(
    (open: boolean) => {
      if (chatId) {
        dispatch(setTaskWidgetExpanded({ id: chatId, expanded: open }));
      }
    },
    [dispatch, chatId],
  );

  if (!everUsed) return null;

  const hasActive = tasks.some((t) => t.status === "in_progress");
  const isAnimating = hasActive && isStreaming;

  return (
    <Box className={styles.widget}>
      <Collapsible.Root open={isExpanded} onOpenChange={handleOpenChange}>
        <Collapsible.Trigger asChild>
          <Flex className={styles.header} align="center" gap="3" px="3" py="2">
            <AnimatedText as="div" size="1" animating={isAnimating}>
              <Flex align="center" gap="2" style={{ flex: 1 }}>
                <CheckboxIcon
                  width={14}
                  height={14}
                  className={styles.headerIcon}
                />

                {!isExpanded && hasTasks && (
                  <>
                    <Flex gap="1" align="center">
                      {tasks.map((task) => (
                        <StatusIcon
                          key={task.id}
                          status={task.status}
                          isStreaming={
                            task.status === "in_progress" && isStreaming
                          }
                        />
                      ))}
                    </Flex>

                    <Text size="1" color="gray">
                      {done}/{total}
                    </Text>

                    <ProgressBar
                      done={done}
                      total={total}
                      animating={isAnimating}
                    />

                    {activeTitle && (
                      <Text size="1" color="gray" className={styles.activeHint}>
                        {activeTitle}
                      </Text>
                    )}
                  </>
                )}

                {!isExpanded && !hasTasks && (
                  <Text size="1" color="gray">
                    Tasks cleared
                  </Text>
                )}

                {isExpanded && (
                  <Text size="1" weight="medium">
                    Task Progress
                  </Text>
                )}
              </Flex>
            </AnimatedText>

            <Chevron open={isExpanded} />
          </Flex>
        </Collapsible.Trigger>

        <Collapsible.Content>
          <Flex
            direction="column"
            gap="2"
            px="3"
            pb="3"
            className={styles.content}
          >
            {hasTasks ? (
              <>
                <div className={styles.taskList}>
                  {tasks.map((task, index) => (
                    <div
                      key={task.id}
                      className={styles.taskRowEnter}
                      style={{ animationDelay: `${index * 50}ms` }}
                    >
                      <TaskRow task={task} isStreaming={isStreaming} />
                    </div>
                  ))}
                </div>
                <Separator size="4" />
                <Text size="1" color="gray">
                  {done}/{total} completed
                </Text>
              </>
            ) : (
              <Text size="1" color="gray">
                No active tasks
              </Text>
            )}
          </Flex>
        </Collapsible.Content>
      </Collapsible.Root>
    </Box>
  );
};

export default TaskProgressWidget;
