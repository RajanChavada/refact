import React, { useMemo } from "react";
import { DesktopIcon, ImageIcon } from "@radix-ui/react-icons";
import { Box, Flex } from "@radix-ui/themes";
import { ToolCard, ToolStatus } from "./ToolCard";
import { useStoredOpen } from "../useStoredOpen";
import { useAppSelector } from "../../../hooks";
import { selectToolResultById } from "../../../features/Chat/Thread/selectors";
import { ToolCall } from "../../../services/refact/types";
import { ShikiCodeBlock } from "../../Markdown";
import { DialogImage } from "../../DialogImage";
import styles from "./ChromeTool.module.css";

interface ChromeArgs {
  commands?: string;
}

interface ChromeToolProps {
  toolCall: ToolCall;
}

function extractFirstNavigateUrl(commands: string): string | null {
  for (const line of commands.split("\n")) {
    const parts = line.trim().split(/\s+/);
    if (parts[0] === "navigate_to" && parts.length >= 3) {
      return parts[2];
    }
  }
  return null;
}

function countScreenshots(commands: string): number {
  return commands
    .split("\n")
    .filter((line) => line.trim().startsWith("screenshot ")).length;
}

export const ChromeTool: React.FC<ChromeToolProps> = ({ toolCall }) => {
  const storeKey = toolCall.id ? `tc:${toolCall.id}` : undefined;
  const [isOpen, handleToggle] = useStoredOpen(storeKey);

  const maybeResult = useAppSelector((state) =>
    selectToolResultById(state, toolCall.id),
  );

  const args = useMemo((): ChromeArgs => {
    try {
      return JSON.parse(toolCall.function.arguments) as ChromeArgs;
    } catch {
      return {};
    }
  }, [toolCall.function.arguments]);

  const status: ToolStatus = useMemo(() => {
    if (!maybeResult) return "running";
    if (
      typeof maybeResult === "object" &&
      "tool_failed" in maybeResult &&
      maybeResult.tool_failed
    ) {
      return "error";
    }
    return "success";
  }, [maybeResult]);

  const { textLog, images } = useMemo(() => {
    if (!maybeResult) return { textLog: null, images: [] as string[] };

    const content = maybeResult.content;

    if (typeof content === "string") {
      return { textLog: content || null, images: [] as string[] };
    }

    if (!Array.isArray(content)) {
      return { textLog: null, images: [] as string[] };
    }

    const textParts = content
      .filter((item) => item.m_type === "text")
      .map((item) => item.m_content)
      .join("\n")
      .trim();

    const imageParts = content
      .filter((item) => item.m_type.startsWith("image/"))
      .map((item) => `data:${item.m_type};base64,${item.m_content}`);

    return { textLog: textParts || null, images: imageParts };
  }, [maybeResult]);

  const summary = useMemo(() => {
    const cmdStr = args.commands ?? "";
    const url = extractFirstNavigateUrl(cmdStr);
    const screenshotCount = countScreenshots(cmdStr);

    const urlLabel = url ? (
      <span className={styles.url}>{url.replace(/^file:\/\//, "")}</span>
    ) : null;

    const screenshotLabel =
      screenshotCount > 0 ? (
        <span className={styles.meta}>
          {screenshotCount} screenshot{screenshotCount !== 1 ? "s" : ""}
        </span>
      ) : null;

    if (urlLabel && screenshotLabel) {
      return (
        <>
          Browser {urlLabel} · {screenshotLabel}
        </>
      );
    }
    if (urlLabel) return <>Browser {urlLabel}</>;
    if (screenshotLabel) return <>Browser · {screenshotLabel}</>;
    return <>Browser commands</>;
  }, [args]);

  const icon = images.length > 0 ? <ImageIcon /> : <DesktopIcon />;

  return (
    <ToolCard
      icon={icon}
      summary={summary}
      status={status}
      isOpen={isOpen}
      onToggle={handleToggle}
      toolCall={toolCall}
    >
      {images.length > 0 && (
        <Flex py="2" gap="2" wrap="wrap">
          {images.map((url, idx) => (
            <DialogImage key={idx} src={url} fallback="" size="8" />
          ))}
        </Flex>
      )}
      {textLog && (
        <Box className={styles.logContent}>
          <ShikiCodeBlock showLineNumbers={false}>{textLog}</ShikiCodeBlock>
        </Box>
      )}
    </ToolCard>
  );
};

export default ChromeTool;
