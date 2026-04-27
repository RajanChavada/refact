import React, { useCallback, useState } from "react";
import {
  Dialog,
  Flex,
  Text,
  Button,
  Callout,
  Badge,
  Spinner,
} from "@radix-ui/themes";
import { ExclamationTriangleIcon } from "@radix-ui/react-icons";
import { useApplyModeTransitionMutation } from "../../services/refact/trajectory";
import { trajectoriesApi } from "../../services/refact/trajectories";
import {
  createChatWithId,
  requestSseRefresh,
  closeThread,
} from "../../features/Chat/Thread/actions";
import {
  openTask,
  addPlannerChat,
  setTaskActiveChat,
} from "../../features/Tasks/tasksSlice";
import { push } from "../../features/Pages/pagesSlice";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { selectLspPort, selectApiKey } from "../../features/Config/configSlice";
import { regenerate } from "../../services/refact/chatCommands";
import {
  useCreateTaskMutation,
  useCreatePlannerChatMutation,
} from "../../services/refact/tasks";
import styles from "./ModeTransitionDialog.module.css";

function extractErrorMessage(err: unknown): string {
  if (err && typeof err === "object") {
    const obj = err as Record<string, unknown>;
    if (obj.data && typeof obj.data === "object") {
      const data = obj.data as Record<string, unknown>;
      if (typeof data.detail === "string") return data.detail;
    }
    if (typeof obj.data === "string") return obj.data;
    if (typeof obj.message === "string") return obj.message;
  }
  if (err instanceof Error) return err.message;
  return "Failed to create task planner";
}

type TaskPlannerDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  chatId: string;
  hasMessages: boolean;
  taskId?: string;
};

export const TaskPlannerDialog: React.FC<TaskPlannerDialogProps> = ({
  open,
  onOpenChange,
  chatId,
  hasMessages,
  taskId,
}) => {
  const dispatch = useAppDispatch();
  const port = useAppSelector(selectLspPort);
  const apiKey = useAppSelector(selectApiKey);
  const [error, setError] = useState<string | null>(null);

  const [applyMutation, { isLoading: isApplying }] =
    useApplyModeTransitionMutation();
  const [createTask, { isLoading: isCreatingTask }] = useCreateTaskMutation();
  const [createPlannerChat, { isLoading: isCreatingPlanner }] =
    useCreatePlannerChatMutation();

  const isInTaskWorkspace = taskId !== undefined;
  const isLoading = isApplying || isCreatingTask || isCreatingPlanner;

  const handleApply = useCallback(async () => {
    setError(null);
    const now = new Date().toISOString();
    try {
      if (isInTaskWorkspace && taskId) {
        let newChatId: string;

        if (hasMessages && chatId) {
          const result = await applyMutation({
            chatId,
            targetMode: "task_planner",
            targetModeDescription:
              "Create and manage tasks with structured planning",
          }).unwrap();
          newChatId = result.new_chat_id;

          await dispatch(
            trajectoriesApi.endpoints.listAllTrajectories.initiate(undefined, {
              forceRefetch: true,
            }),
          ).unwrap();

          dispatch(
            createChatWithId({
              id: newChatId,
              mode: "task_planner",
              parentId: chatId,
              linkType: "mode_transition",
            }),
          );
        } else {
          const result = await createPlannerChat(taskId).unwrap();
          newChatId = result.chat_id;
          dispatch(createChatWithId({ id: newChatId, mode: "task_planner" }));
        }

        dispatch(requestSseRefresh({ chatId: newChatId }));
        dispatch(
          addPlannerChat({
            taskId,
            planner: {
              id: newChatId,
              title: "",
              createdAt: now,
              updatedAt: now,
            },
          }),
        );
        dispatch(
          setTaskActiveChat({
            taskId,
            activeChat: { type: "planner", chatId: newChatId },
          }),
        );
        onOpenChange(false);

        if (hasMessages && chatId) {
          void regenerate(newChatId, port, apiKey ?? undefined);
        }
      } else {
        const task = await createTask({ name: "New Task" }).unwrap();
        const newTaskId = task.id;
        let newChatId: string;

        if (hasMessages && chatId) {
          const result = await applyMutation({
            chatId,
            targetMode: "task_planner",
            targetModeDescription:
              "Create and manage tasks with structured planning",
          }).unwrap();
          newChatId = result.new_chat_id;

          await dispatch(
            trajectoriesApi.endpoints.listAllTrajectories.initiate(undefined, {
              forceRefetch: true,
            }),
          ).unwrap();

          dispatch(closeThread({ id: chatId, force: true }));
          dispatch(
            createChatWithId({
              id: newChatId,
              mode: "task_planner",
              parentId: chatId,
              linkType: "mode_transition",
            }),
          );
        } else {
          const result = await createPlannerChat(newTaskId).unwrap();
          newChatId = result.chat_id;
          dispatch(createChatWithId({ id: newChatId, mode: "task_planner" }));
        }

        dispatch(requestSseRefresh({ chatId: newChatId }));
        dispatch(openTask({ id: newTaskId, name: task.name }));
        dispatch(
          addPlannerChat({
            taskId: newTaskId,
            planner: {
              id: newChatId,
              title: "",
              createdAt: now,
              updatedAt: now,
            },
          }),
        );
        dispatch(
          setTaskActiveChat({
            taskId: newTaskId,
            activeChat: { type: "planner", chatId: newChatId },
          }),
        );
        dispatch(push({ name: "task workspace", taskId: newTaskId }));
        onOpenChange(false);

        if (hasMessages && chatId) {
          void regenerate(newChatId, port, apiKey ?? undefined);
        }
      }
    } catch (err) {
      setError(extractErrorMessage(err));
    }
  }, [
    isInTaskWorkspace,
    taskId,
    chatId,
    hasMessages,
    applyMutation,
    createTask,
    createPlannerChat,
    dispatch,
    onOpenChange,
    port,
    apiKey,
  ]);

  const handleOpenChange = useCallback(
    (newOpen: boolean) => {
      if (!newOpen) setError(null);
      onOpenChange(newOpen);
    },
    [onOpenChange],
  );

  const title = isInTaskWorkspace ? "New Planner" : "Switch to Task Planner";
  const description = isInTaskWorkspace
    ? hasMessages
      ? "The assistant will analyze the current planner and create a new planner chat with the relevant context."
      : "Create a new planner chat in this task."
    : hasMessages
      ? "The assistant will analyze your conversation, create a new task, and start a planner chat with the relevant context."
      : "Create a new task and open the Task Planner.";
  const buttonLabel = isInTaskWorkspace ? "Create Planner" : "Create Task";
  const loadingLabel = isApplying
    ? "Analyzing..."
    : isCreatingTask
      ? "Creating task..."
      : "Creating planner...";

  return (
    <Dialog.Root open={open} onOpenChange={handleOpenChange}>
      <Dialog.Content maxWidth="500px" className={styles.dialogContent}>
        <Dialog.Title>
          <Flex align="center" gap="2">
            <Text>{title}</Text>
            <Badge color="blue">task_planner</Badge>
          </Flex>
        </Dialog.Title>

        <Dialog.Description size="2" color="gray">
          {description}
        </Dialog.Description>

        {error && (
          <Callout.Root color="red" className={styles.callout}>
            <Callout.Icon>
              <ExclamationTriangleIcon />
            </Callout.Icon>
            <Callout.Text>{error}</Callout.Text>
          </Callout.Root>
        )}

        {isLoading && (
          <Flex
            align="center"
            justify="center"
            gap="2"
            className={styles.loadingContainer}
          >
            <Spinner />
            <Text color="gray">{loadingLabel}</Text>
          </Flex>
        )}

        <Flex gap="3" mt="4" justify="end">
          <Dialog.Close>
            <Button variant="soft" color="gray" disabled={isLoading}>
              Cancel
            </Button>
          </Dialog.Close>
          <Button onClick={() => void handleApply()} disabled={isLoading}>
            {isLoading ? (
              <>
                <Spinner size="1" />
                {loadingLabel}
              </>
            ) : (
              buttonLabel
            )}
          </Button>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
};

TaskPlannerDialog.displayName = "TaskPlannerDialog";
