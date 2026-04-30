<a name="readme-top"></a>

<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://docs.refact.ai/_astro/logo-dark.CCzD55EA.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://docs.refact.ai/_astro/logo-light.CblxRz3x.svg">
    <img alt="Refact logo" src="https://docs.refact.ai/_astro/logo-dark.CCzD55EA.svg" width="200">
  </picture>
  <h1 align="center">Refact</h1>
  <p align="center">Open-source, local-first AI coding assistant for IDE chat, autonomous agent workflows, and code completion.</p>
</div>

<div align="center">
  <a href="https://github.com/smallcloudai/refact/stargazers"><img src="https://img.shields.io/github/stars/smallcloudai/refact?style=for-the-badge&color=blue" alt="GitHub stars"></a>
  <a href="https://docs.refact.ai"><img src="https://img.shields.io/badge/documentation-blue?logo=googledocs&logoColor=FFE165&style=for-the-badge" alt="Documentation"></a>
  <a href="https://github.com/smallcloudai/refact/issues"><img src="https://img.shields.io/badge/issues-github?style=for-the-badge" alt="GitHub issues"></a>
</div>

Refact runs a local Rust engine (`refact-lsp`) from your IDE and connects only to the BYOK providers or local model runtimes you configure. The supported path is local-first: install the IDE plugin, choose your model provider, and keep project context, indexes, trajectories, and task state on your machine.

## What Refact does

- **IDE chat** with project-aware context, file mentions, selected code, images, and streaming responses.
- **Autonomous agent mode** that can inspect, edit, test, and iterate on code with confirmation controls.
- **Code completion** powered by the local engine, workspace files, AST context, and provider capabilities.
- **Local code intelligence** through AST indexes, semantic search, workspace tree/search tools, and file-aware prompts.
- **Provider choice** across BYOK APIs, OpenAI-compatible endpoints, hosted provider APIs, and local runtimes such as Ollama, LM Studio, and vLLM.
- **Tool integrations** for GitHub, GitLab, Bitbucket, PostgreSQL, MySQL, PDB, command-line tools, long-running services, browser automation, and MCP.
- **Knowledge, tasks, and trajectories** for reusable project memory, Kanban-style agent work, checkpoints, and resumable chat history.

## Quickstart

1. **Install an IDE plugin**
   - VS Code: follow the [VS Code installation guide](https://docs.refact.ai/installation/vs-code/).
   - JetBrains IDEs: follow the [JetBrains installation guide](https://docs.refact.ai/installation/jetbrains/).
2. **Open a workspace** and launch the Refact sidebar or tool window. The plugin starts the local `refact-lsp` engine on localhost.
3. **Configure a provider** in **Provider Setup**.
   - Add a BYOK provider such as OpenAI, Anthropic, Google Gemini, OpenRouter, DeepSeek, Groq, xAI, Qwen, Kimi, Zhipu, MiniMax, Doubao, GitHub Copilot, Claude Code, or a custom OpenAI-compatible endpoint.
   - Or add a local runtime such as Ollama, LM Studio, or vLLM.
   - Refact discovers available models from the provider/runtime configuration instead of relying on a fixed model list.
4. **Pick defaults** in **Default Models** for chat, agent work, and completion where applicable.
5. **Start using Refact**: ask questions in chat, run agent tasks, request code edits, or accept completions in the editor.

No hosted Refact account, managed inference endpoint, or Refact-issued API key is required for local/BYOK usage.

## Repository map

| Area | Path | Purpose |
| --- | --- | --- |
| Agent Engine | `refact-agent/engine/` | Rust `refact-lsp` HTTP/LSP engine, providers, tools, indexes, integrations |
| Agent GUI | `refact-agent/gui/` | React/Vite chat UI package used by IDE webviews and standalone development |
| VS Code extension | `extra/refact-vscode/` | VS Code host integration |
| JetBrains plugin | `extra/refact-intellij/` | JetBrains host integration |
| Docs site | `docs/` | Astro/Starlight documentation site |

## Developer quick commands

```bash
# Engine
(cd refact-agent/engine && cargo check && cargo test --lib)

# GUI
(cd refact-agent/gui && npm ci && npm run types && npm run lint && npm run test)

# Docs
(cd docs && npm ci && npm run build)
```

See the dedicated READMEs in each subproject for full development workflows.

## Documentation and support

- [Documentation](https://docs.refact.ai/)
- [Provider setup (BYOK)](https://docs.refact.ai/byok/)
- [GitHub issues](https://github.com/smallcloudai/refact/issues)
- [GitHub discussions](https://github.com/smallcloudai/refact/discussions)

## Contributing

Contributions are welcome. Please open an issue or discussion for larger changes, and run the relevant engine, GUI, or docs checks before submitting a pull request.

## License

Refact is distributed under the BSD-3-Clause license. See the repository license for details.
