import React from "react";
import { Flex, Container, Box, Text } from "@radix-ui/themes";
import { ChatContextFile } from "../../services/refact";
import * as Collapsible from "@radix-ui/react-collapsible";
import { Link } from "../Link";
import ReactMarkDown from "react-markdown";
import { MarkdownCodeBlock } from "../Markdown/CodeBlock";
import { Chevron } from "../Collapsible";
import { filename } from "../../utils";
import { useEventsBusForIDE } from "../../hooks";

export const Markdown: React.FC<{
  children: string;
  startingLineNumber?: number;
}> = ({ startingLineNumber, ...props }) => {
  return (
    <ReactMarkDown
      components={{
        code({ style: _style, color: _color, ...codeProps }) {
          return (
            <MarkdownCodeBlock
              {...codeProps}
              showLineNumbers={false}
              startingLineNumber={startingLineNumber}
            />
          );
        },
      }}
      {...props}
    />
  );
};

function getExtensionFromName(name: string): string {
  const dot = name.lastIndexOf(".");
  if (dot === -1) return "";
  return name.substring(dot + 1).replace(/:\d*-\d*/, "");
}

const FilesContent: React.FC<{
  files: ChatContextFile[];
  onOpenFile: (file: { file_path: string; line?: number }) => Promise<void>;
  isEnrichment?: boolean;
}> = ({ files, onOpenFile, isEnrichment = false }) => {
  if (files.length === 0) return null;

  if (isEnrichment) {
    const memories = files.filter(f => f.file_name.includes("/.refact/memories/"));
    const trajectories = files.filter(f => f.file_name.includes("/.refact/trajectories/"));
    const other = files.filter(f =>
      !f.file_name.includes("/.refact/memories/") &&
      !f.file_name.includes("/.refact/trajectories/")
    );

    return (
      <Flex direction="column" gap="2">
        {memories.length > 0 && (
          <FileSection icon="📝" title="Knowledge" files={memories} onOpenFile={onOpenFile} isEnrichment />
        )}
        {trajectories.length > 0 && (
          <FileSection icon="💬" title="Past Conversations" files={trajectories} onOpenFile={onOpenFile} isEnrichment />
        )}
        {other.length > 0 && (
          <FileSection icon="📄" title="Related" files={other} onOpenFile={onOpenFile} isEnrichment />
        )}
      </Flex>
    );
  }

  return (
    <Flex direction="column" gap="1">
      {files.map((file, index) => (
        <FileCard
          key={file.file_name + index}
          file={file}
          onOpenFile={onOpenFile}
          isEnrichment={false}
        />
      ))}
    </Flex>
  );
};

export const ContextFiles: React.FC<{
  files: ChatContextFile[];
  isEnrichment?: boolean;
}> = ({ files, isEnrichment = false }) => {
  const [open, setOpen] = React.useState(false);
  const { queryPathThenOpenFile } = useEventsBusForIDE();

  if (!Array.isArray(files) || files.length === 0) return null;

  const icon = isEnrichment ? "🧠" : "📎";
  const label = isEnrichment
    ? `${files.length} memories`
    : `${files.length} file${files.length > 1 ? "s" : ""}`;

  return (
    <Container>
      <Collapsible.Root open={open} onOpenChange={setOpen}>
        <Collapsible.Trigger asChild>
          <Flex gap="2" align="start" py="2" style={{ cursor: "pointer" }}>
            <Text weight="light" size="1" style={{ color: "var(--gray-10)" }}>
              {icon} {label}
            </Text>
            <Chevron open={open} />
          </Flex>
        </Collapsible.Trigger>
        <Collapsible.Content>
          <FilesContent
            files={files}
            onOpenFile={queryPathThenOpenFile}
            isEnrichment={isEnrichment}
          />
        </Collapsible.Content>
      </Collapsible.Root>
    </Container>
  );
};

const FileSection: React.FC<{
  icon: string;
  title: string;
  files: ChatContextFile[];
  onOpenFile: (file: { file_path: string; line?: number }) => Promise<void>;
  isEnrichment?: boolean;
}> = ({ icon, title, files, onOpenFile, isEnrichment }) => {
  return (
    <Box>
      <Text size="1" weight="light" style={{ color: "var(--gray-9)" }}>
        {icon} {title}
      </Text>
      <Flex direction="column" gap="1" mt="1">
        {files.map((file, index) => (
          <FileCard
            key={file.file_name + index}
            file={file}
            onOpenFile={onOpenFile}
            isEnrichment={isEnrichment}
          />
        ))}
      </Flex>
    </Box>
  );
};

const FileCard: React.FC<{
  file: ChatContextFile;
  onOpenFile: (file: { file_path: string; line?: number }) => Promise<void>;
  isEnrichment?: boolean;
}> = ({ file, onOpenFile, isEnrichment }) => {
  const [showContent, setShowContent] = React.useState(false);
  const extension = getExtensionFromName(file.file_name);
  const start = file.line1 || 1;

  const displayName = isEnrichment
    ? extractEnrichmentDisplayName(file.file_name)
    : formatFileName(file.file_name, file.line1, file.line2);
  const relevance = file.usefulness ? Math.round(file.usefulness) : null;

  const preview = file.file_content.slice(0, 100).replace(/\n/g, " ") +
    (file.file_content.length > 100 ? "..." : "");

  return (
    <Box pl="2" style={{ borderLeft: "1px solid var(--gray-a4)" }}>
      <Flex justify="between" align="start" gap="2">
        <Box style={{ flex: 1, minWidth: 0 }}>
          <Flex align="center" gap="2">
            <Link
              onClick={(e) => {
                e.preventDefault();
                void onOpenFile({ file_path: file.file_name, line: file.line1 });
              }}
              style={{ cursor: "pointer" }}
            >
              <Text size="1" weight="light" style={{ color: "var(--gray-11)" }}>
                {displayName}
              </Text>
            </Link>
            {relevance !== null && (
              <Text size="1" style={{ color: "var(--gray-9)" }}>
                {relevance}%
              </Text>
            )}
          </Flex>
          <Text size="1" style={{ color: "var(--gray-9)" }}>
            {preview}
          </Text>
        </Box>
        <Box
          style={{ cursor: "pointer", flexShrink: 0 }}
          onClick={() => setShowContent(!showContent)}
        >
          <Chevron open={showContent} />
        </Box>
      </Flex>
      {showContent && (
        <Box mt="2" style={{ maxHeight: "300px", overflow: "auto" }}>
          <Markdown startingLineNumber={start}>
            {"```" + extension + "\n" + file.file_content + "\n```"}
          </Markdown>
        </Box>
      )}
    </Box>
  );
};

function formatFileName(filePath: string, line1?: number, line2?: number): string {
  const name = filename(filePath);
  if (line1 && line2 && line1 !== 0 && line2 !== 0) {
    return `${name}:${line1}-${line2}`;
  }
  return name;
}

function extractEnrichmentDisplayName(filePath: string): string {
  const fileName = filename(filePath);

  // Memory files: 2025-12-26_230536_3fe00894_servicebobpy-is-a-standalone-fastapi.md
  // Extract the readable part after the hash
  const memoryMatch = fileName.match(/^\d{4}-\d{2}-\d{2}_\d{6}_[a-f0-9]+_(.+)\.md$/);
  if (memoryMatch) {
    return memoryMatch[1].replace(/-/g, " ");
  }

  // Trajectory files: UUID.json - show as "Conversation"
  const trajectoryMatch = fileName.match(/^[a-f0-9-]{36}\.json$/);
  if (trajectoryMatch) {
    return "Past conversation";
  }

  return fileName;
}
