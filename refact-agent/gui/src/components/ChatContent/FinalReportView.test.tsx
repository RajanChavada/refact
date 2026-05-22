import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "../../utils/test-utils";
import { FinalReportView } from "./FinalReportView";

const structuredPayload = JSON.stringify({
  summary: "Implemented **structured** final report rendering.",
  success: true,
  files_changed: ["src/components/ChatContent/FinalReportView.tsx"],
  tests_added_or_updated: ["FinalReportView.test.tsx"],
  verification: [
    {
      command: "npm run test -- FinalReportView --run",
      exit_code: 0,
      passed: true,
      output_tail: "test passed",
    },
    {
      command: "npm run lint",
      exit_code: 1,
      passed: false,
      output_tail: "lint failed",
    },
  ],
  followup_cards: [
    {
      title: "Add create-card actions",
      priority: "P2",
      instructions: "Add optional follow-up card creation controls later.",
    },
  ],
  risks: ["Renderer only handles the current structured schema."],
  assumptions: ["Legacy markdown remains available."],
});

const unsafeStructuredPayload = JSON.stringify({
  summary: [
    "Rendered untrusted markdown.",
    "<script>window.x=1</script>",
    '<img onerror="alert(1)" src=x>',
  ].join("\n\n"),
  success: true,
  files_changed: [],
  tests_added_or_updated: [],
  verification: [],
  followup_cards: [],
  risks: [],
  assumptions: [],
});

describe("FinalReportView", () => {
  it("renders structured payload sections", () => {
    render(<FinalReportView content={structuredPayload} title="Card T-1" />);
    expect(screen.getByText("Card T-1")).toBeInTheDocument();
    expect(screen.getByText(/Success/)).toBeInTheDocument();
    expect(screen.getByText("structured")).toBeInTheDocument();
    expect(
      screen.getByText("src/components/ChatContent/FinalReportView.tsx"),
    ).toBeInTheDocument();
    expect(screen.getByText("FinalReportView.test.tsx")).toBeInTheDocument();
    expect(screen.getByText("Followup cards")).toBeInTheDocument();
    expect(screen.getByText("Add create-card actions")).toBeInTheDocument();
  });

  it("does not render raw scripts or inline image handlers from summary markdown", () => {
    const { container } = render(
      <FinalReportView content={unsafeStructuredPayload} />,
    );

    expect(container.querySelector("script")).not.toBeInTheDocument();
    expect(container.querySelector("img[onerror]")).not.toBeInTheDocument();
  });

  it("shows verification pass and fail emoji", () => {
    render(<FinalReportView content={structuredPayload} />);
    expect(screen.getAllByText("✅").length).toBeGreaterThan(0);
    expect(screen.getAllByText("❌").length).toBeGreaterThan(0);
    expect(
      screen.getByText("npm run test -- FinalReportView --run"),
    ).toBeInTheDocument();
    expect(screen.getByText("npm run lint")).toBeInTheDocument();
  });

  it("renders legacy string payload without error", () => {
    render(<FinalReportView content="Legacy report\n\nStill readable." />);
    expect(screen.getByText(/Legacy report/)).toBeInTheDocument();
    expect(screen.getByText(/Still readable/)).toBeInTheDocument();
  });

  it("renders minimal structured payload", () => {
    render(
      <FinalReportView
        content={JSON.stringify({ summary: "Minimal report", success: true })}
      />,
    );

    expect(screen.getByText("Minimal report")).toBeInTheDocument();
    expect(screen.getByText(/Success/)).toBeInTheDocument();
    expect(screen.getByText("Files changed")).toBeInTheDocument();
    expect(screen.getByText("Tests added or updated")).toBeInTheDocument();
    expect(screen.getByText("Verification")).toBeInTheDocument();
    expect(screen.getByText("Followup cards")).toBeInTheDocument();
    expect(screen.getByText("Risks")).toBeInTheDocument();
    expect(screen.getByText("Assumptions")).toBeInTheDocument();
    expect(screen.getAllByText("None")).toHaveLength(6);
  });

  it("renders null optional fields as empty sections", () => {
    render(
      <FinalReportView
        content={JSON.stringify({
          summary: "Null optional fields report",
          success: false,
          files_changed: null,
          tests_added_or_updated: null,
          verification: null,
          followup_cards: null,
          risks: null,
          assumptions: null,
        })}
      />,
    );

    expect(screen.getByText("Null optional fields report")).toBeInTheDocument();
    expect(screen.getByText(/Failed/)).toBeInTheDocument();
    expect(screen.getByText("Files changed")).toBeInTheDocument();
    expect(screen.getByText("Tests added or updated")).toBeInTheDocument();
    expect(screen.getByText("Verification")).toBeInTheDocument();
    expect(screen.getByText("Followup cards")).toBeInTheDocument();
    expect(screen.getByText("Risks")).toBeInTheDocument();
    expect(screen.getByText("Assumptions")).toBeInTheDocument();
    expect(screen.getAllByText("None")).toHaveLength(6);
  });

  it("renders followup cards read-only", () => {
    render(<FinalReportView content={structuredPayload} />);
    expect(screen.getByText("Add create-card actions")).toBeInTheDocument();
    expect(screen.getByText("P2")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /create/i }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("+")).not.toBeInTheDocument();
  });

  it("keeps verification output tails expandable", () => {
    render(<FinalReportView content={structuredPayload} />);
    expect(screen.getByText("test passed")).toBeInTheDocument();
    fireEvent.click(screen.getByText("npm run test -- FinalReportView --run"));
    expect(screen.getByText("test passed")).toBeInTheDocument();
  });
});
