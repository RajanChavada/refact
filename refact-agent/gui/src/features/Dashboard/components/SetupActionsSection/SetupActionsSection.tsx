import React, { useCallback } from "react";
import { Text } from "@radix-ui/themes";
import { ChevronDownIcon, ChevronUpIcon } from "@radix-ui/react-icons";
import { useAppDispatch } from "../../../../hooks";
import { openChatInModeAndStart } from "../../../Chat/Thread/actions";
import { CollapsePanel } from "../../../../components/shared/CollapsePanel";
import styles from "./SetupActionsSection.module.css";

type SetupAction = {
  label: string;
  mode: string;
};

const SETUP_ACTIONS: SetupAction[] = [
  { label: "Create Skills", mode: "setup_skills" },
  { label: "Setup AGENTS.md", mode: "setup_agents_md" },
  { label: "Find MCPs", mode: "setup_mcp" },
  { label: "Create Commands", mode: "setup_commands" },
  { label: "Create Subagents", mode: "setup_subagents" },
];

type Props = {
  collapsed: boolean;
  onToggleCollapsed: () => void;
};

export const SetupActionsSection: React.FC<Props> = ({
  collapsed,
  onToggleCollapsed,
}) => {
  const dispatch = useAppDispatch();

  const openSetupChat = useCallback(
    (mode: string) => {
      void dispatch(openChatInModeAndStart({ mode }));
    },
    [dispatch],
  );

  return (
    <div className={styles.section} data-collapsed={collapsed || undefined}>
      <button
        type="button"
        className={styles.headerToggle}
        onClick={onToggleCollapsed}
        aria-expanded={!collapsed}
      >
        <Text size="1" weight="bold" color="gray" className={styles.label}>
          PROJECT SETUP
        </Text>
        {collapsed ? (
          <ChevronDownIcon width={12} height={12} color="var(--gray-9)" />
        ) : (
          <ChevronUpIcon width={12} height={12} color="var(--gray-9)" />
        )}
      </button>
      <CollapsePanel collapsed={collapsed}>
        <div className={styles.buttons}>
          {SETUP_ACTIONS.map((action) => (
            <button
              key={action.mode}
              type="button"
              className={styles.button}
              onClick={() => openSetupChat(action.mode)}
            >
              <Text size="1">{action.label}</Text>
            </button>
          ))}
        </div>
      </CollapsePanel>
    </div>
  );
};
