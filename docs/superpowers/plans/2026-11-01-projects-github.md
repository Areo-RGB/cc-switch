# Projects + GitHub Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Phase 3 Projects + GitHub native backend without taking ownership of Phase 2’s project catalog or MCP/Skills commands.

**Architecture:** A new `src-tauri/src/project_backend/` feature module ports the pure DesktopCC logic and a new `src-tauri/src/commands/projects_github.rs` maps it to Tauri commands. Shared registration changes are limited to one contiguous `// projects-github` block in `commands/mod.rs` and `lib.rs`.

**Tech Stack:** Rust 2021, Tauri 2 commands/events, serde/serde_json, std process/filesystem APIs, existing `tauri-plugin-opener`, Vitest TypeScript API wrappers.

**Spec:** `docs/superpowers/specs/2026-11-01-projects-github-design.md`

## Global Constraints

- Do not implement project listing/hiding/adding or MCP/Skills commands; Phase 2 owns them.
- Keep command JSON fields camelCase and match `docs/project-ui-commands.md`.
- Never create a real GitHub repository or push in tests.
- Long-running scripts stream events and kill their complete process group.
- Feature implementation lives in new files; shared files receive only additive registration lines.

## File Map

- `src-tauri/src/project_backend/mod.rs`: public feature types and project inspection/summary orchestration.
- `src-tauri/src/project_backend/git.rs`: Git command execution, porcelain parsing, safe paths, and Git mutations.
- `src-tauri/src/project_backend/github.rs`: `gh` command runner, auth/repo parsing/filtering, remote helpers.
- `src-tauri/src/project_backend/launchers.rs`: folder/editor/terminal launchers and manifest-based script detection.
- `src-tauri/src/project_backend/process.rs`: executable lookup, streamed child output, process-group termination.
- `src-tauri/src/project_backend/local_repos.rs`: GitHub slug parsing and bounded local repository scanning.
- `src-tauri/src/commands/projects_github.rs`: Tauri command adapters, process registry, and Tauri events.
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`: additive command/module registration only.
- `src/lib/api/github.ts`: frontend types/wrappers for the phase’s GitHub auth/list operations.
- `src/lib/api/git.ts`: add the native diff wrapper while preserving existing project-page calls.
- `docs/project-ui-commands.md`: document the additional Git diff/auth/repo-list command contracts.

## Tasks

### 1. Establish failing parser/detection tests

- [x] Add unit tests for Git status parsing, script detection, GitHub JSON/filter/name/URL parsing, and local-repository slug scanning.
- [x] Run the focused Rust tests and confirm they fail because the new feature module/functions do not exist.

### 2. Implement pure feature modules

- [x] Implement `process.rs` with PATH resolution, own process groups, output streaming, and group kill.
- [x] Implement `launchers.rs` with safe command parsing, open-folder/editor/terminal launch, project inspection, and script detection for package.json, Cargo, just, Python, Make, Go, Maven, and Gradle markers.
- [x] Implement `git.rs` with the DesktopCC Git behavior required by the contract, including safe repository paths and diff handling.
- [x] Implement `github.rs` and `local_repos.rs` with `gh` CLI calls/helpers and no implicit network fallback.
- [x] Run the feature unit tests and the full Rust test suite.

### 3. Implement Tauri command adapters

- [x] Add project metadata/open/script/agent-summary commands.
- [x] Add Git status/branches/diff/stage/unstage/commit/checkout/remote commands.
- [x] Add GitHub open/create/link-push/auth-status/repo-list commands.
- [x] Emit script output/exit events and terminate all registered processes on app exit.
- [x] Register the feature module and commands in the two shared files.
- [x] Add command-facing tests that exercise serialization and fake-command behavior without network side effects.

### 4. Wire frontend contracts

- [x] Add the Git diff wrapper and GitHub auth/list API wrappers/types.
- [x] Remove only Phase 3-owned project/Git/GitHub entries from dev mocks if they are no longer used by the native API layer; retain Phase 2-owned project catalog and MCP/Skills fixtures.
- [x] Extend the command contract documentation for the added commands.

### 5. Verify and hand off

- [x] Run `cargo check`.
- [x] Run `cargo test`.
- [x] Run `cargo clippy --all-targets`.
- [x] Run `pnpm typecheck` and `pnpm build:renderer`.
- [x] Run one bounded `pnpm tauri dev` smoke check and inspect logs.
- [x] Review the diff, commit conventionally, and push the branch.
