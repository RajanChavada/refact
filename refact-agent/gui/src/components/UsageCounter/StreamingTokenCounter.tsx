import React, { useMemo, useEffect, useRef, useState } from "react";
import { Flex, Text } from "@radix-ui/themes";
import classNames from "classnames";

import { useAppSelector } from "../../hooks";
import {
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectThreadMaximumTokens,
} from "../../features/Chat";
import {
  AssistantMessage,
  isAssistantMessage,
  isUserMessage,
} from "../../services/refact";
import { formatNumberToFixed } from "../../utils/formatNumberToFixed";
import { useUsageCounter } from "./useUsageCounter";

import styles from "./StreamingTokenCounter.module.css";

function estimateTokens(text: string): number {
  if (!text) return 0;
  return Math.ceil(text.length / 4);
}

function findLastIndex<T>(arr: T[], pred: (x: T) => boolean): number {
  for (let i = arr.length - 1; i >= 0; i--) {
    if (pred(arr[i])) return i;
  }
  return -1;
}

function extractAllText(message: AssistantMessage | null): string {
  if (!message) return "";

  let text = message.content ?? "";

  if (message.reasoning_content) {
    text += message.reasoning_content;
  }

  if (message.thinking_blocks) {
    for (const block of message.thinking_blocks) {
      if (block.thinking) text += block.thinking;
      if (block.signature) text += block.signature;
    }
  }

  return text;
}

export const StreamingTokenCounter: React.FC = () => {
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const messages = useAppSelector(selectMessages);
  const maxContextTokens = useAppSelector(selectThreadMaximumTokens) ?? 0;

  const { currentSessionTokens } = useUsageCounter();

  const [visible, setVisible] = useState(() => isStreaming || isWaiting);

  const [displayTokens, setDisplayTokens] = useState(0);
  const [pulseKey, setPulseKey] = useState(0);
  const prevTokensRef = useRef(0);

  const lastAssistantIdx = useMemo(
    () => findLastIndex(messages, isAssistantMessage),
    [messages],
  );
  const lastUserIdx = useMemo(
    () => findLastIndex(messages, isUserMessage),
    [messages],
  );

  const waitingForNewAssistant =
    (isWaiting || isStreaming) && lastUserIdx > lastAssistantIdx;

  const activeAssistantMessage = useMemo((): AssistantMessage | null => {
    if (waitingForNewAssistant) return null; // Show gray fallback context
    if (lastAssistantIdx < 0) return null;
    return messages[lastAssistantIdx] as AssistantMessage;
  }, [messages, lastAssistantIdx, waitingForNewAssistant]);

  const usage = activeAssistantMessage?.usage;

  const allText = useMemo(
    () => extractAllText(activeAssistantMessage),
    [activeAssistantMessage],
  );

  const actualOutputTokens = usage?.completion_tokens ?? 0;

  const estimatedOutputTokens = useMemo(() => {
    return estimateTokens(allText);
  }, [allText]);

  const outputTokens =
    actualOutputTokens > 0 ? actualOutputTokens : estimatedOutputTokens;

  const actualContextTokens = usage?.prompt_tokens ?? 0;
  const contextTokens =
    actualContextTokens > 0 ? actualContextTokens : currentSessionTokens;

  const isFallbackContext = actualContextTokens === 0 && contextTokens > 0;

  const contextPercentage = useMemo(() => {
    if (!maxContextTokens || maxContextTokens === 0) return 0;
    return Math.round((contextTokens / maxContextTokens) * 100);
  }, [contextTokens, maxContextTokens]);

  useEffect(() => {
    setVisible(isStreaming || isWaiting);
  }, [isStreaming, isWaiting]);

  useEffect(() => {
    if (outputTokens !== prevTokensRef.current) {
      prevTokensRef.current = outputTokens;
      setDisplayTokens(outputTokens);
      setPulseKey((k) => k + 1);
    }
  }, [outputTokens]);

  useEffect(() => {
    if (!isStreaming && !isWaiting) {
      setDisplayTokens(0);
      prevTokensRef.current = 0;
    }
  }, [isStreaming, isWaiting]);

  if (!visible) return null;

  const showPlaceholder = allText.length === 0 && (isStreaming || isWaiting);

  const isOutputEstimate = actualOutputTokens === 0;

  return (
    <Flex align="center" gap="1" className={styles.inlineContainer}>
      {/* Separator */}
      <Text className={styles.separator}>|</Text>

      {/* Output tokens (live counter or placeholder) */}
      <Text
        key={pulseKey}
        className={classNames(styles.tokenValue, {
          [styles.animateValue]: displayTokens > 0,
        })}
      >
        {showPlaceholder
          ? "…"
          : `${isOutputEstimate ? "~" : ""}${formatNumberToFixed(displayTokens)}`}
      </Text>

      {/* Context percentage if available */}
      {contextTokens > 0 && maxContextTokens > 0 && (
        <Text
          className={classNames(styles.contextPercent, {
            [styles.fallback]: isFallbackContext,
            [styles.warning]: contextPercentage >= 70,
            [styles.critical]: contextPercentage >= 90,
          })}
        >
          ({isOutputEstimate || isFallbackContext ? "~" : ""}
          {contextPercentage}%)
        </Text>
      )}
    </Flex>
  );
};
