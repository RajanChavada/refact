import type { ComponentProps, FC, ReactNode } from "react";
import { Flex, Text } from "@radix-ui/themes";
import {
  ImageIcon,
  CursorArrowIcon,
  RocketIcon,
  GearIcon,
  BarChartIcon,
  EyeOpenIcon,
  WidthIcon,
  ScissorsIcon,
} from "@radix-ui/react-icons";
import type { ModelCapabilities } from "../utils/groupModelsWithPricing";
import styles from "../ModelCard.module.css";

export type CapabilityIconsProps = {
  capabilities?: ModelCapabilities;
  size?: "1" | "2";
};

type ModelDetailIconProps = {
  icon: ReactNode;
  children?: ReactNode;
  color?: ComponentProps<typeof Text>["color"];
  tone?: "default" | "accent";
};

export const ModelDetailIcon: FC<ModelDetailIconProps> = ({
  icon,
  children,
  color = "gray",
  tone = "default",
}) => (
  <Text as="span" size="1" color={color}>
    <Flex as="span" align="center" gap="1" className={styles.modelDetailIcon}>
      <span
        className={
          tone === "accent"
            ? styles.modelDetailIconGlyphAccent
            : styles.modelDetailIconGlyph
        }
      >
        {icon}
      </span>
      {children}
    </Flex>
  </Text>
);

type DetailSvgIconProps = ComponentProps<typeof WidthIcon>;

export const ContextWindowIcon: FC<DetailSvgIconProps> = (props) => (
  <WidthIcon {...props} />
);
export const MaxOutputIcon: FC<DetailSvgIconProps> = (props) => (
  <ScissorsIcon {...props} />
);
export const PricingIcon: FC<DetailSvgIconProps> = (props) => (
  <BarChartIcon {...props} />
);
export const ToolsIcon: FC<DetailSvgIconProps> = (props) => (
  <GearIcon {...props} />
);
export const VisionIcon: FC<DetailSvgIconProps> = (props) => (
  <EyeOpenIcon {...props} />
);
export const ReasoningIcon: FC<DetailSvgIconProps> = (props) => (
  <svg
    width="15"
    height="15"
    viewBox="0 0 15 15"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
    {...props}
  >
    <path
      d="M6.25 2.45C5.5 1.85 4.25 2 3.85 3.05C2.85 3.1 2.1 3.9 2.2 4.9C1.45 5.35 1.3 6.55 1.95 7.2C1.55 8.25 2.15 9.35 3.25 9.6C3.45 10.8 4.85 11.2 6.25 10.45V2.45Z"
      stroke="currentColor"
      strokeWidth="1.2"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
    <path
      d="M8.75 2.45C9.5 1.85 10.75 2 11.15 3.05C12.15 3.1 12.9 3.9 12.8 4.9C13.55 5.35 13.7 6.55 13.05 7.2C13.45 8.25 12.85 9.35 11.75 9.6C11.55 10.8 10.15 11.2 8.75 10.45V2.45Z"
      stroke="currentColor"
      strokeWidth="1.2"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
    <path
      d="M6.25 4.4H5.05M6.25 6.55H4.8M6.25 8.7H5.25M8.75 4.4H9.95M8.75 6.55H10.2M8.75 8.7H9.75"
      stroke="currentColor"
      strokeWidth="1.2"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
  </svg>
);

export const CapabilityIcons: FC<CapabilityIconsProps> = ({
  capabilities,
  size = "1",
}) => {
  if (!capabilities) return null;

  const iconSize = size === "1" ? 12 : 14;
  const iconStyle = { width: iconSize, height: iconSize };

  return (
    <Flex gap="1" align="center">
      {capabilities.supportsTools && (
        <span title="Supports tools">
          <ToolsIcon style={iconStyle} color="var(--gray-11)" />
        </span>
      )}
      {capabilities.supportsMultimodality && (
        <span title="Supports images">
          <ImageIcon style={iconStyle} color="var(--gray-11)" />
        </span>
      )}
      {capabilities.supportsClicks && (
        <span title="Computer use">
          <CursorArrowIcon style={iconStyle} color="var(--gray-11)" />
        </span>
      )}
      {capabilities.supportsAgent && (
        <span title="Agent mode">
          <RocketIcon style={iconStyle} color="var(--gray-11)" />
        </span>
      )}
      {(!!capabilities.reasoningEffortOptions?.length ||
        !!capabilities.supportsThinkingBudget ||
        !!capabilities.supportsAdaptiveThinkingBudget) && (
        <span title="Reasoning">
          <ReasoningIcon style={iconStyle} color="var(--blue-11)" />
        </span>
      )}
    </Flex>
  );
};
