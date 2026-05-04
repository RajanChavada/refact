# Refact Monorepo

AI coding assistant: Rust engine (LSP/HTTP server) + React chat UI + IDE plugins (VSCode, JetBrains) + cloud backend.

## Repository Map

| Subproject | Path | Language | AGENTS.md |
|---|---|---|---|
| Agent Engine | `refact-agent/engine/` | Rust 2021, async/tokio | ✅ `refact-agent/engine/AGENTS.md` |
| Agent GUI | `refact-agent/gui/` | TypeScript/React 18 | ✅ `refact-agent/gui/AGENTS.md` |
| VSCode Extension | `extra/refact-vscode/` | TypeScript | — |
| JetBrains Plugin | `extra/refact-intellij/` | Kotlin, Gradle | — |
| Cloud Backend | `extra/web_v1_backend/` | Python 3.10, FastAPI | — |
| Documentation | `docs/` | Astro (static site) | — |

Sub-project `AGENTS.md` files contain detailed architecture, patterns, and checklists. Read them before working in those directories.

## Verification Commands

**Always verify your changes compile and pass tests before finishing.** Both engine and GUI builds are heavy — plan accordingly.

### Engine (`refact-agent/engine/`)

```bash
cd refact-agent/engine

# Fast check — type/borrow errors only (~1-3 min, no codegen)
cargo check

# Unit + doc tests (~3-8 min first build, ~1-3 min incremental)
cargo test --lib && cargo test --doc

# Full release build (~10-20 min cold, ~2-5 min incremental)
# LTO + opt-level=z + strip — very slow from scratch
cargo build --release
```

⚠️ **First build compiles ~85 crates + 7 tree-sitter parsers + SQLite. Expect 10-20 minutes cold.** Incremental builds are much faster. CI runs `cargo test --release` on 7 platform targets.

Python integration tests (`tests/*.py`) require a running `refact-lsp` instance — don't run them as a quick check.

### GUI (`refact-agent/gui/`)

```bash
cd refact-agent/gui

# All CI checks (~1-3 min total)
npm run test              # vitest (unit, excludes integration)
npm run format:check      # prettier — no code changes
npm run types             # tsc --noEmit
npm run lint              # eslint, 0 warnings allowed

# Full build (~30-60s)
npm run build
```

⚠️ **ESLint is strict-type-checked with `--max-warnings 0`.** Any new warning fails CI. Run `npm run lint` before committing TypeScript changes.

### Minimum pre-commit checks

If you changed **only engine Rust code**: `cd refact-agent/engine && cargo check && cargo test --lib`
If you changed **only GUI TypeScript**: `cd refact-agent/gui && npm run types && npm run lint && npm run test`
If you changed **both**: run both sets.

## CI Quality Gates (GitHub Actions)

| Workflow | Trigger paths | Checks |
|---|---|---|
| `agent_engine_build` | `refact-agent/engine/**` | `cargo test --release` on 7 targets (Win/Linux/macOS × x86_64/aarch64) |
| `agent_gui_build` | `refact-agent/gui/**` | `npm test` → `format:check` → `types` → `lint` → `build` (Node LTS + latest) |
| `server_build` | `refact-server/**` | Docker multi-arch build |
| `docs_build` | `docs/**` | Docker build + push |

## Architecture Overview

```
┌─────────────────┐     postMessage      ┌──────────────────┐
│  IDE Plugins    │◄────────────────────►│   Agent GUI      │
│  (VSCode/JB)    │                      │   (React webview)│
└────────┬────────┘                      └────────┬─────────┘
         │ LSP (stdin/stdout)                     │ HTTP + SSE
         │ or HTTP                                │
         └──────────────┬─────────────────────────┘
                        ▼
              ┌─────────────────────┐
              │   Agent Engine      │
              │   (refact-lsp)      │
              │   HTTP :8001 + LSP  │
              └──────┬──────────────┘
                     │
         ┌───────────┼───────────────┐
         ▼           ▼               ▼
    LLM APIs    Local indexes    Integrations
   (15+ providers) (AST, VecDB)  (GitHub, MCP, shell, etc.)
```

- **Engine ↔ GUI**: HTTP REST + SSE streaming (`/v1/chats/subscribe`). GUI sends commands via `POST /v1/chats/{id}/commands`, receives state via SSE events with monotonic `seq` numbers.
- **Engine ↔ IDE**: LSP protocol (tower-lsp) for completions/code-lens, plus HTTP for chat and tools.
- **IDE ↔ GUI**: `postMessage` bridge (VSCode `acquireVsCodeApi`, JetBrains `postIntellijMessage`). Events: file context, theme, tool calls.

## Cross-Project Conventions

### Rust (Engine)

- **Formatting**: `rustfmt.toml` — 100 char lines, 4-space indent, Unix newlines, `reorder_imports = false`.
- **Async discipline**: All shared state through `GlobalContext` (`Arc<ARwLock<>>`). Drop read guards before `.await`. Never hold `gcx.read()` across await points.
- **Shutdown**: Check `shutdown_flag.load(Ordering::Relaxed)` in loops. Use `select!` with shutdown arm for channel receivers. Never `loop { sleep }` without a shutdown check. Store `JoinHandle` for spawned tasks — no fire-and-forget `tokio::spawn`.
- **Lock ordering**: Always acquire `gcx` ARwLock before inner mutexes. Reversing order risks deadlocks in background threads.
- **Error handling**: `Result<>` with contextual errors. `.ok_or_else()` over `.unwrap()` for runtime data.

### TypeScript/React (GUI)

- **Linting**: ESLint strict-type-checked, 0 warnings. Prettier enforced in CI.
- **State**: Redux Toolkit + RTK Query. Always use selectors from `features/Chat/Thread/selectors.ts`. Never access `state.chat.threads[id]` directly.
- **Styling**: Radix UI primitives + CSS Modules + design tokens. No inline styles, no hardcoded colors, no magic numbers.
- **File naming**: `PascalCase.tsx` (components), `useCamelCase.ts` (hooks), `camelCase.ts` (utils), `PascalCase.module.css`.
- **No `any` types.**

### Kotlin (JetBrains Plugin)

- Java 17 target. Gradle build with IntelliJ Platform Plugin. Communicates with engine via HTTP + JCEF webview for chat.

### Python (Backend)

- Python 3.10+. FastAPI + Uvicorn. Type hints expected.

## Project Config Locations

| Scope | Path | Contents |
|---|---|---|
| User config | `~/.config/refact/` | `default_privacy.yaml`, `providers.d/*.yaml` |
| Cache | `~/.cache/refact/` | Shadow repos, logs, telemetry, integrations |
| Project | `.refact/` | `trajectories/`, `knowledge/`, `tasks/`, `integrations.d/` |
| System prompts | `refact-agent/engine/yaml_configs/defaults/` | Modes, subagents, toolbox commands |

### AGENTS.md Scoping Rules

AGENTS.md files can appear at any directory level. Scope = entire directory tree rooted at that folder. More-deeply-nested files take precedence on conflicts. Direct user instructions override all AGENTS.md content.

## Common Pitfalls

- **Shutdown hangs**: `loop {}` without `shutdown_flag`, bare `.recv().await`/`.changed().await` without `select!` + timeout, `tokio::spawn` without stored handle.
- **Lock inversion**: `gcx.read().await` → inner mutex is safe order. Reversing (inner mutex → gcx) causes deadlocks under load.
- **SSE sequence gaps**: Every event has monotonic `seq`. Gap → client reconnects for fresh snapshot. Never skip or reorder events.
- **Thinking block signatures**: Anthropic thinking blocks with cryptographic signatures must be preserved byte-for-byte. No JSON rebuilding, no field reordering.
- **GUI state**: Chat/history state is ephemeral (not persisted). Only `tour` and `userSurvey` survive Redux persist.
