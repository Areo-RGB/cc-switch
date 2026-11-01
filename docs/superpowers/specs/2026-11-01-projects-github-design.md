# Projects + GitHub Backend Design

## Intent

Provide the native backend for the existing Projects detail page: inspect a
project, detect runnable scripts, launch the folder/editor/terminal, expose
Git status and mutations, and use the `gh` CLI for GitHub state and actions.
Long-running scripts stream stdout/stderr to Tauri events and are terminated
as process groups when stopped or when the app exits.

## Scope

Included: project detail metadata, script detection for common manifests,
launchers, process management, local Git repository scanning helpers, Git
status/branches/diff/stage/unstage/commit/checkout/remote, GitHub auth and
filtered repository listing, GitHub open/create/link-and-push, and the
documented Tauri command surface.

Excluded: project listing, hiding, and adding; MCP/Skills project commands;
real GitHub API calls in tests; UI redesign; destructive GitHub operations not
requested by the command contract.

## Approaches

1. Bridge to DesktopCC or Python: rejected because it would add a runtime
   dependency and make Tauri event/process ownership indirect.
2. Copy the GPUI modules wholesale: rejected because the current app has a
   different state and command model and the source modules contain GPUI-only
   globals.
3. Port the pure logic into a new Tauri feature module: chosen. It keeps the
   existing UI contract, isolates phase-owned code, and allows unit tests to
   exercise parsing and filesystem behavior without invoking GitHub.

## Architecture

`src-tauri/src/project_backend/` contains pure-ish project, Git, GitHub, launcher,
process, and local-repository modules. `src-tauri/src/commands/projects_github.rs`
adapts them to Tauri commands and owns event payloads. Script processes are
registered in a feature-local synchronized registry; each process is spawned
in its own OS process group and its output reader threads emit line events.

The project ID is the canonical project path, matching the existing frontend
mock and the Phase 2 project model. Command responses use camelCase JSON and
the exact types in `docs/project-ui-commands.md`; additional `git_diff`,
`github_auth_status`, and `github_repo_list` commands cover the phase brief.

## Verification

Unit tests cover Git porcelain parsing and safe paths, script detection,
GitHub JSON/filter/name/URL helpers, local-repository scanning, process
streaming/termination primitives, and command-facing serialization. Full
verification is `cargo check`, `cargo test`, `cargo clippy --all-targets`,
`pnpm typecheck`, `pnpm build:renderer`, and one bounded `pnpm tauri dev`
smoke run. Tests use temporary repositories and fake executables only.
