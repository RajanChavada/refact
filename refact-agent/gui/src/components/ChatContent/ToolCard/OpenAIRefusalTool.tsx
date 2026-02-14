import React, { useMemo } from "react";
import { ExclamationTriangleIcon } from "@radix-ui/react-icons";
import { Box, Text } from "@radix-ui/themes";

import { ToolCard } from "./ToolCard";
import type { ToolCall } from "../../../services/refact/types";
import { ShikiCodeBlock } from "../../Markdown";
import styles from "./OpenAIResponsesTool.module.css";
import { useOpenAiResponsesToolCardState } from "./openaiResponsesToolCardState";

type Props = {
  toolCall: ToolCall;
};

export const OpenAIRefusalTool: React.FC<Props> = ({ toolCall }) => {
  const state = useOpenAiResponsesToolCardState(toolCall);
  const args = state.parsedArgs as Record<string, unknown> | null;

  const refusal = useMemo(() => {
    if (!args) return null;
    if (typeof (args as any).refusal === "string") return (args as any).refusal as string;
    if (typeof (args as any).text === "string") return (args as any).text as string;
    return null;
  }, [args]);

  return (
    <ToolCard
      icon={<ExclamationTriangleIcon />}
      summary={"Refusal"}
      status={state.status}
      isOpen={state.isOpen}
      onToggle={state.toggleOpen}
      toolCall={toolCall}
    >
      {refusal && (
        <Box className={styles.codeBox}>
          <ShikiCodeBlock showLineNumbers={false}>{refusal}</ShikiCodeBlock>
        </Box>
      )}

      <Text size="1" color="gray">
        Raw JSON
      </Text>
      <ShikiCodeBlock showLineNumbers={false}>{state.rawJson}</ShikiCodeBlock>
    </ToolCard>
  );
};

