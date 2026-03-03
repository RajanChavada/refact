import React, { useCallback } from "react";
import { Button, Flex, Text } from "@radix-ui/themes";
import { CalloutFromTop } from "../../components/Callout/Callout";
import { useGetSetupStatusQuery } from "../../services/refact/setupStatus";
import { useAppDispatch } from "../../hooks/useAppDispatch";
import { createChatWithId } from "../Chat/Thread/actions";
import { push } from "../Pages/pagesSlice";
import styles from "./SetupBanner.module.css";

export const SetupBanner: React.FC = () => {
  const dispatch = useAppDispatch();
  const { data, isError } = useGetSetupStatusQuery(undefined, {
    refetchOnMountOrArgChange: true,
  });

  const openSetupChat = useCallback(() => {
    dispatch(createChatWithId({ id: globalThis.crypto.randomUUID(), mode: "setup" }));
    dispatch(push({ name: "chat" }));
  }, [dispatch]);

  if (isError || !data || data.configured) return null;

  return (
    <CalloutFromTop>
      <Flex direction={{ initial: "column", sm: "row" }} gap="3" align="center">
        <Text size="2" className={styles.text}>
          This project isn’t set up for Refact yet. Run setup to generate
          guidelines, integrations, and toolbox commands.
        </Text>
        <Button size="2" onClick={openSetupChat} className={styles.banner}>
          Run Setup
        </Button>
      </Flex>
    </CalloutFromTop>
  );
};
