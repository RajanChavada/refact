import React, { useMemo } from "react";

import { Flex } from "@radix-ui/themes";

import { BuddyCanvas, useBuddyState } from "../../features/Buddy";

const BUDDY_HELLOS = [
  "Hi! I'm Buddy. What should we build today?",
  "Hello! I'm ready when you are.",
  "Hey, I'm Buddy. Tell me what's on your mind.",
  "Hi there! Want to explore some code together?",
  "Hello from Buddy. Let's make something nice.",
];

const pickHello = () =>
  BUDDY_HELLOS[Math.floor(Math.random() * BUDDY_HELLOS.length)];

export const PlaceHolderText: React.FC = () => {
  const buddy = useBuddyState();
  const speech = useMemo(pickHello, []);

  return (
    <Flex
      direction="column"
      align="center"
      justify="center"
      width="100%"
      height="100%"
      minHeight="100%"
    >
      <BuddyCanvas
        state={buddy.state}
        onEvent={buddy.handleCanvasEvent}
        displaySize={220}
        speechOverride={speech}
        bubblePosition="top"
      />
    </Flex>
  );
};
