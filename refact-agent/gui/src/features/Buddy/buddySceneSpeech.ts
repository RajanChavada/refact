import type {
  BuddyControl,
  BuddyRuntimeEvent,
  BuddySpeechItem,
  BuddySuggestion,
} from "./types";

export type BuddySceneSpeechSource = "speech" | "runtime" | "suggestion";

export interface BuddySceneSpeech {
  text: string;
  controls: BuddyControl[];
  chat_id?: string;
  source: BuddySceneSpeechSource;
  runtimeEventId?: string;
}

function runtimeEventText(event: BuddyRuntimeEvent): string {
  const speechText = event.speech_text?.trim();
  if (speechText) return speechText;

  const description = event.description?.trim();
  if (description) {
    return `${event.title}: ${description}`;
  }

  return event.title;
}

function runtimeEventToSpeech(
  event: BuddyRuntimeEvent | null | undefined,
): BuddySceneSpeech | null {
  if (!event || event.dismissed) return null;
  const text = runtimeEventText(event).trim();
  if (!text) return null;
  return {
    text,
    controls: event.controls ?? [],
    chat_id: event.chat_id,
    source: "runtime",
    runtimeEventId: event.id,
  };
}

function suggestionToSpeech(
  suggestion: BuddySuggestion | null | undefined,
): BuddySceneSpeech | null {
  if (!suggestion || suggestion.dismissed) return null;
  return {
    text: `${suggestion.title}: ${suggestion.description}`,
    controls: suggestion.controls.map((control) =>
      control.action === "dismiss"
        ? {
            ...control,
            action: "dismiss_suggestion",
            action_param: suggestion.id,
          }
        : control,
    ),
    source: "suggestion",
  };
}

function runtimeFromQueue(
  nowPlaying: BuddyRuntimeEvent | null,
  runtimeQueue: BuddyRuntimeEvent[],
): BuddyRuntimeEvent | null {
  const candidates = [nowPlaying, ...runtimeQueue].filter(
    (event): event is BuddyRuntimeEvent =>
      event !== null &&
      !event.dismissed &&
      runtimeEventText(event).trim() !== "",
  );

  return candidates.sort(compareRuntimeEvents)[0] ?? null;
}

function runtimePriorityScore(event: BuddyRuntimeEvent): number {
  const priorityScore = (() => {
    switch (event.priority) {
      case "critical":
        return 400;
      case "high":
        return 300;
      case "normal":
        return 100;
      case "low":
        return 0;
      default:
        return 50;
    }
  })();

  const statusScore = (() => {
    switch (event.status) {
      case "failed":
        return 500;
      case "started":
      case "progress":
      case "streaming":
        return 300;
      case "info":
        return 150;
      case "completed":
        return 25;
    }
  })();

  const hasControlsScore = event.controls?.length ? 20 : 0;
  return priorityScore + statusScore + hasControlsScore;
}

function runtimeCreatedAtMs(event: BuddyRuntimeEvent): number {
  const timestamp = Date.parse(event.created_at);
  return Number.isFinite(timestamp) ? timestamp : 0;
}

function compareRuntimeEvents(
  left: BuddyRuntimeEvent,
  right: BuddyRuntimeEvent,
): number {
  const scoreDiff = runtimePriorityScore(right) - runtimePriorityScore(left);
  if (scoreDiff !== 0) return scoreDiff;
  return runtimeCreatedAtMs(right) - runtimeCreatedAtMs(left);
}

export function buildBuddySceneSpeech(args: {
  activeSpeech: BuddySpeechItem | null;
  nowPlaying: BuddyRuntimeEvent | null;
  runtimeQueue: BuddyRuntimeEvent[];
  activeSuggestion?: BuddySuggestion | null;
}): BuddySceneSpeech | null {
  if (args.activeSpeech) {
    return {
      text: args.activeSpeech.text,
      controls: args.activeSpeech.controls,
      chat_id: args.activeSpeech.chat_id,
      source: "speech",
    };
  }

  return (
    runtimeEventToSpeech(
      runtimeFromQueue(args.nowPlaying, args.runtimeQueue),
    ) ?? suggestionToSpeech(args.activeSuggestion)
  );
}
