import React, { useCallback, useMemo, useState } from "react";
import {
  Badge,
  Box,
  Button,
  Callout,
  Card,
  Checkbox,
  Flex,
  IconButton,
  Popover,
  Select,
  Spinner,
  Text,
  Tooltip,
} from "@radix-ui/themes";
import {
  ClockIcon,
  DrawingPinIcon,
  ExclamationTriangleIcon,
  Pencil2Icon,
  PlusIcon,
  TrashIcon,
} from "@radix-ui/react-icons";
import classNames from "classnames";
import { Markdown } from "../../../components/Markdown";
import { DocumentEditor } from "./DocumentEditor";
import {
  type TaskDocumentKind,
  type TaskDocumentSummary,
  useDeleteTaskDocumentMutation,
  useGetTaskDocumentHistoryQuery,
  useGetTaskDocumentQuery,
  useListTaskDocumentsQuery,
  usePinTaskDocumentMutation,
} from "../../../services/refact/taskDocumentsApi";
import styles from "./DocumentsPanel.module.css";

const ALL_VALUE = "all";

const DOCUMENT_KINDS: TaskDocumentKind[] = [
  "plan",
  "design",
  "runbook",
  "brief",
  "postmortem",
  "spec",
];

const KIND_COLORS: Record<
  TaskDocumentKind,
  "blue" | "purple" | "green" | "teal" | "red" | "gray"
> = {
  plan: "blue",
  design: "purple",
  runbook: "green",
  brief: "teal",
  postmortem: "red",
  spec: "gray",
};

function formatUpdatedAt(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function optimisticKey(taskId: string, slug: string): string {
  return `${taskId}:${slug}`;
}

type DocumentRowProps = {
  document: TaskDocumentSummary;
  isExpanded: boolean;
  expandedContent?: string;
  optimisticPinned?: boolean;
  onToggleExpand: () => void;
  onPin: (slug: string, pinned: boolean) => void | Promise<void>;
  onEdit: (slug: string) => void;
  onHistory: (slug: string) => void;
  onDelete: (slug: string) => void | Promise<void>;
};

const DocumentRow: React.FC<DocumentRowProps> = ({
  document,
  isExpanded,
  expandedContent,
  optimisticPinned,
  onToggleExpand,
  onPin,
  onEdit,
  onHistory,
  onDelete,
}) => {
  const pinned = optimisticPinned ?? document.pinned;

  return (
    <Card
      className={classNames(styles.row, pinned && styles.rowPinned)}
      data-testid={`document-row-${document.slug}`}
      onClick={onToggleExpand}
    >
      <Flex justify="between" align="start" gap="2">
        <Flex align="center" gap="2" wrap="wrap" className={styles.rowHeader}>
          <Tooltip content={pinned ? "Unpin" : "Pin"}>
            <IconButton
              size="1"
              variant="ghost"
              color={pinned ? "amber" : "gray"}
              aria-label={pinned ? "Unpin" : "Pin"}
              className={styles.rowIconButton}
              onClick={(e) => {
                e.stopPropagation();
                void onPin(document.slug, !pinned);
              }}
            >
              <DrawingPinIcon />
            </IconButton>
          </Tooltip>
          <Badge
            color={KIND_COLORS[document.kind]}
            variant="soft"
            size="1"
            data-testid={`kind-badge-${document.slug}`}
          >
            {document.kind}
          </Badge>
          <Text weight="bold" size="2">
            {document.name}
          </Text>
          <Text size="1" color="gray">
            v{document.version}
          </Text>
          <Text size="1" color="gray">
            {formatUpdatedAt(document.updated_at)}
          </Text>
        </Flex>

        <Flex gap="1" align="center" className={styles.rowControls}>
          <Tooltip content="Edit">
            <IconButton
              size="1"
              variant="ghost"
              color="gray"
              aria-label="Edit"
              className={styles.rowIconButton}
              onClick={(e) => {
                e.stopPropagation();
                onEdit(document.slug);
              }}
            >
              <Pencil2Icon />
            </IconButton>
          </Tooltip>
          <Tooltip content="History">
            <IconButton
              size="1"
              variant="ghost"
              color="gray"
              aria-label="History"
              className={styles.rowIconButton}
              onClick={(e) => {
                e.stopPropagation();
                onHistory(document.slug);
              }}
            >
              <ClockIcon />
            </IconButton>
          </Tooltip>
          <Popover.Root>
            <Tooltip content="Delete">
              <Popover.Trigger>
                <IconButton
                  size="1"
                  variant="ghost"
                  color="red"
                  aria-label="Delete"
                  className={styles.rowIconButton}
                  onClick={(e) => e.stopPropagation()}
                >
                  <TrashIcon />
                </IconButton>
              </Popover.Trigger>
            </Tooltip>
            <Popover.Content width="220px">
              <Flex direction="column" gap="3">
                <Text size="2">Delete this document?</Text>
                <Flex gap="2">
                  <Popover.Close>
                    <Button
                      size="1"
                      variant="solid"
                      color="red"
                      onClick={() => {
                        void onDelete(document.slug);
                      }}
                    >
                      Confirm delete
                    </Button>
                  </Popover.Close>
                  <Popover.Close>
                    <Button size="1" variant="soft" color="gray">
                      Cancel
                    </Button>
                  </Popover.Close>
                </Flex>
              </Flex>
            </Popover.Content>
          </Popover.Root>
        </Flex>
      </Flex>

      {isExpanded && (
        <Box className={styles.content}>
          {expandedContent !== undefined ? (
            <Markdown canHaveInteractiveElements={false}>
              {expandedContent}
            </Markdown>
          ) : (
            <Flex justify="center" p="2">
              <Spinner />
            </Flex>
          )}
        </Box>
      )}
    </Card>
  );
};

type DocumentsPanelProps = {
  taskId: string;
};

export const DocumentsPanel: React.FC<DocumentsPanelProps> = ({ taskId }) => {
  const [kindFilter, setKindFilter] = useState<string>(ALL_VALUE);
  const [pinnedOnly, setPinnedOnly] = useState(false);
  const [expandedSlug, setExpandedSlug] = useState<string | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editorMode, setEditorMode] = useState<"create" | "edit">("create");
  const [editorSlug, setEditorSlug] = useState<string | undefined>(undefined);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [historySlug, setHistorySlug] = useState<string | null>(null);
  const [optimisticPinned, setOptimisticPinned] = useState<
    ReadonlyMap<string, boolean>
  >(() => new Map());

  const { data, isFetching, error } = useListTaskDocumentsQuery({ taskId });

  const { data: expandedDoc } = useGetTaskDocumentQuery(
    { taskId, slug: expandedSlug ?? "" },
    { skip: !expandedSlug },
  );

  const { data: historyData } = useGetTaskDocumentHistoryQuery(
    { taskId, slug: historySlug ?? "" },
    { skip: !historySlug || !historyOpen },
  );

  const [pinDocument] = usePinTaskDocumentMutation();
  const [deleteDocument] = useDeleteTaskDocumentMutation();

  const documentsWithOptimistic = useMemo(() => {
    const docs = data?.documents ?? [];
    return docs.map((doc) => ({
      ...doc,
      pinned:
        optimisticPinned.get(optimisticKey(taskId, doc.slug)) ?? doc.pinned,
    }));
  }, [data?.documents, optimisticPinned, taskId]);

  const sorted = useMemo(() => {
    return [...documentsWithOptimistic].sort((a, b) => {
      if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
      return b.updated_at.localeCompare(a.updated_at);
    });
  }, [documentsWithOptimistic]);

  const visible = useMemo(() => {
    return sorted.filter((doc) => {
      if (kindFilter !== ALL_VALUE && doc.kind !== kindFilter) return false;
      if (pinnedOnly && !doc.pinned) return false;
      return true;
    });
  }, [sorted, kindFilter, pinnedOnly]);

  const handleToggleExpand = useCallback((slug: string) => {
    setExpandedSlug((prev) => (prev === slug ? null : slug));
  }, []);

  const handlePin = useCallback(
    async (slug: string, pinned: boolean) => {
      const key = optimisticKey(taskId, slug);
      setOptimisticPinned((prev) => new Map(prev).set(key, pinned));
      try {
        await pinDocument({ taskId, slug, pinned }).unwrap();
      } catch {
        setOptimisticPinned((prev) => new Map(prev).set(key, !pinned));
      }
    },
    [pinDocument, taskId],
  );

  const handleEdit = useCallback((slug: string) => {
    setEditorSlug(slug);
    setEditorMode("edit");
    setEditorOpen(true);
  }, []);

  const handleHistory = useCallback((slug: string) => {
    setHistorySlug(slug);
    setHistoryOpen(true);
  }, []);

  const handleDelete = useCallback(
    async (slug: string) => {
      await deleteDocument({ taskId, slug })
        .unwrap()
        .catch(() => undefined);
    },
    [deleteDocument, taskId],
  );

  const handleNewDocument = useCallback(() => {
    setEditorSlug(undefined);
    setEditorMode("create");
    setEditorOpen(true);
  }, []);

  return (
    <Box className={styles.root}>
      <Flex justify="between" align="center" gap="2" className={styles.header}>
        <Text weight="bold" size="3">
          {data?.documents.length ?? 0} documents
        </Text>
        <Button size="1" variant="soft" onClick={handleNewDocument}>
          <PlusIcon />
          New
        </Button>
      </Flex>

      <Flex gap="2" align="center" className={styles.filters} wrap="wrap">
        <Select.Root value={kindFilter} onValueChange={setKindFilter} size="1">
          <Select.Trigger
            aria-label="Kind filter"
            className={styles.filterControl}
          />
          <Select.Content>
            <Select.Item value={ALL_VALUE}>All kinds</Select.Item>
            {DOCUMENT_KINDS.map((k) => (
              <Select.Item key={k} value={k}>
                {k}
              </Select.Item>
            ))}
          </Select.Content>
        </Select.Root>
        <Text as="label" size="2">
          <Flex align="center" gap="1">
            <Checkbox
              size="1"
              checked={pinnedOnly}
              onCheckedChange={(v) => setPinnedOnly(v === true)}
            />
            Pinned only
          </Flex>
        </Text>
        {isFetching && <Spinner size="1" />}
      </Flex>

      {error && (
        <Callout.Root color="red" size="1">
          <Callout.Icon>
            <ExclamationTriangleIcon />
          </Callout.Icon>
          <Callout.Text>Failed to load documents.</Callout.Text>
        </Callout.Root>
      )}

      <Flex direction="column" gap="2" className={styles.list}>
        {isFetching && !data ? (
          <Flex justify="center" p="4">
            <Spinner />
          </Flex>
        ) : visible.length > 0 ? (
          visible.map((doc) => (
            <DocumentRow
              key={doc.slug}
              document={doc}
              isExpanded={expandedSlug === doc.slug}
              expandedContent={
                expandedSlug === doc.slug ? expandedDoc?.content : undefined
              }
              optimisticPinned={optimisticPinned.get(
                optimisticKey(taskId, doc.slug),
              )}
              onToggleExpand={() => handleToggleExpand(doc.slug)}
              onPin={handlePin}
              onEdit={handleEdit}
              onHistory={handleHistory}
              onDelete={handleDelete}
            />
          ))
        ) : (
          <Text color="gray" size="2" className={styles.emptyState}>
            No documents yet. Click + New to create a plan or design doc.
          </Text>
        )}
      </Flex>

      <DocumentEditor
        taskId={taskId}
        mode={editorMode}
        slug={editorSlug}
        open={editorOpen}
        onOpenChange={setEditorOpen}
      />

      {historyOpen && historySlug && (
        <Box
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.4)",
            zIndex: 100,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}
          onClick={() => setHistoryOpen(false)}
        >
          <Card
            style={{ minWidth: 320, maxWidth: 480, maxHeight: "80vh" }}
            onClick={(e) => e.stopPropagation()}
          >
            <Flex direction="column" gap="3">
              <Text weight="bold" size="3">
                History: {historySlug}
              </Text>
              {historyData?.history.length ? (
                historyData.history.map((entry) => (
                  <Box key={entry.version}>
                    <Text size="2" weight="medium">
                      v{entry.version}
                    </Text>
                    <Text size="1" color="gray" as="div">
                      {formatUpdatedAt(entry.updated_at)}
                    </Text>
                  </Box>
                ))
              ) : (
                <Text size="2" color="gray">
                  No history available.
                </Text>
              )}
              <Flex justify="end">
                <Button
                  size="1"
                  variant="soft"
                  onClick={() => setHistoryOpen(false)}
                >
                  Close
                </Button>
              </Flex>
            </Flex>
          </Card>
        </Box>
      )}
    </Box>
  );
};

export default DocumentsPanel;
