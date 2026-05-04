import React from "react";
import { Box, Card, Flex, Text } from "@radix-ui/themes";
import { ExclamationTriangleIcon } from "@radix-ui/react-icons";
import { Markdown } from "../Markdown";
import styles from "./ChatContent.module.css";

export type ErrorMessageCardProps = {
  errors: string[];
};

export const ErrorMessageCard: React.FC<ErrorMessageCardProps> = ({
  errors,
}) => {
  const title =
    errors.length === 1
      ? "Generation error"
      : `${errors.length} generation errors`;

  return (
    <Card className={styles.errorMessageCard} variant="surface">
      <Flex direction="column" gap="2">
        <Flex align="center" gap="2">
          <Box className={styles.errorMessageIcon}>
            <ExclamationTriangleIcon width="15" height="15" />
          </Box>
          <Text size="2" weight="medium" color="red">
            {title}
          </Text>
        </Flex>
        <Flex direction="column" gap="2">
          {errors.map((error, index) => (
            <Box key={`${index}-${error}`} className={styles.errorMessageBody}>
              <Markdown>{error}</Markdown>
            </Box>
          ))}
        </Flex>
      </Flex>
    </Card>
  );
};
