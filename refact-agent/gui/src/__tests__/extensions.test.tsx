import { render, screen } from "../utils/test-utils";
import { http, HttpResponse } from "msw";
import { describe, expect, it } from "vitest";
import { server } from "../utils/mockServer";
import { ExtItemList } from "../features/Extensions/components/ExtItemList";
import { SkillEditor } from "../features/Extensions/components/SkillEditor";
import type { SkillRegistryItem } from "../services/refact/extensions";

const MOCK_ITEMS: SkillRegistryItem[] = [
  {
    name: "my_skill",
    description: "A global skill",
    source: "global",
    source_label: "Global",
    scope: "global",
    read_only: false,
    file_path: "/home/.config/refact/skills/my_skill/SKILL.md",
  },
  {
    name: "local_skill",
    description: "A local project skill",
    source: "local",
    source_label: "Local",
    scope: "local",
    read_only: false,
    file_path: "/project/.refact/skills/local_skill/SKILL.md",
  },
  {
    name: "plugin_skill",
    description: "A plugin skill",
    source: "plugin:my-plugin",
    source_label: "my-plugin",
    scope: "plugin",
    read_only: true,
    file_path: "/home/.config/refact/plugins/installed/my-plugin/skills/plugin_skill/SKILL.md",
  },
];

describe("ExtItemList", () => {
  it("renders items with correct source badges", () => {
    render(
      <ExtItemList
        items={MOCK_ITEMS}
        selectedId={null}
        onSelect={() => undefined}
        onCreate={() => undefined}
        onDelete={() => undefined}
      />,
    );

    expect(screen.getByText("my_skill")).toBeDefined();
    expect(screen.getByText("local_skill")).toBeDefined();
    expect(screen.getByText("plugin_skill")).toBeDefined();

    expect(screen.getByText("Global")).toBeDefined();
    expect(screen.getByText("Local")).toBeDefined();
    expect(screen.getByText("Plugin")).toBeDefined();
  });

  it("shows delete button only for non-read-only items", () => {
    render(
      <ExtItemList
        items={MOCK_ITEMS}
        selectedId={null}
        onSelect={() => undefined}
        onCreate={() => undefined}
        onDelete={() => undefined}
      />,
    );

    expect(screen.getByLabelText("Delete my_skill")).toBeDefined();
    expect(screen.getByLabelText("Delete local_skill")).toBeDefined();
    expect(screen.queryByLabelText("Delete plugin_skill")).toBeNull();
  });

  it("marks selected item", () => {
    const { container } = render(
      <ExtItemList
        items={MOCK_ITEMS}
        selectedId="my_skill"
        onSelect={() => undefined}
        onCreate={() => undefined}
        onDelete={() => undefined}
      />,
    );

    const selectedEl = container.querySelector('[aria-label="Select my_skill"]');
    expect(selectedEl?.className).toContain("selected");
  });

  it("renders empty state when no items", () => {
    render(
      <ExtItemList
        items={[]}
        selectedId={null}
        onSelect={() => undefined}
        onCreate={() => undefined}
        onDelete={() => undefined}
      />,
    );
    expect(screen.getByText("No items found")).toBeDefined();
  });
});

describe("SkillEditor", () => {
  it("renders form fields reflecting loaded skill data", async () => {
    server.use(
      http.get("http://127.0.0.1:8001/v1/ext/skills/my_skill", () => {
        return HttpResponse.json({
          name: "my_skill",
          description: "A test skill",
          user_invocable: true,
          disable_model_invocation: false,
          allowed_tools: ["shell"],
          model: null,
          context: null,
          agent: null,
          argument_hint: "[arg]",
          body: "# My Skill\nDo something.",
          raw_content: "---\ndescription: A test skill\n---\n# My Skill\nDo something.",
          source: "global",
          file_path: "/home/.config/refact/skills/my_skill/SKILL.md",
        });
      }),
    );

    render(
      <SkillEditor
        name="my_skill"
        onBack={() => undefined}
      />,
      {
        preloadedState: {
          config: {
            apiKey: "test",
            lspPort: 8001,
            themeProps: {},
            host: "vscode",
            addressURL: "Refact",
          },
        },
      },
    );

    const nameInput = await screen.findByDisplayValue("my_skill");
    expect(nameInput).toBeDefined();

    const description = await screen.findByDisplayValue("A test skill");
    expect(description).toBeDefined();
  });
});
