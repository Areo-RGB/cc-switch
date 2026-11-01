# Project UI command contract

The frontend calls these Tauri 2 commands through `src/lib/api/`. Rust should
keep the JSON field names below (camelCase) so the UI can move from the
browser mock to the native data layer without a view change. The browser mock
is in `src/lib/api/devMocks.ts`; mutations are intentionally render-only there.

## Shared types

```ts
type ProjectKind = "git" | "folder" | "workspace";
type Client = "claude" | "codex" | "opencode" | "pi" | "gemini";

interface Project {
  id: string;
  path: string;
  name: string;
  kind: ProjectKind;
  frameworks: string[];
  branch: string;
  lastCommit: string;
  lastCommitAt: string;
  changedFiles: number;
  hidden: boolean;
  remote: { connected: boolean; url?: string; name?: string };
  scripts: ProjectScript[];
  agents: AgentsSummary;
}
interface ProjectScript {
  id: string;
  name: string;
  command: string;
  running: boolean;
}
interface AgentsSummary {
  mcpServers: number;
  skills: number;
  clients: Array<{ client: string; mcpServers: number; skills: number }>;
}
interface GitStatus {
  branch: string;
  ahead: number;
  behind: number;
  entries: Array<{
    path: string;
    status: "new" | "modified" | "deleted" | "renamed";
    staged: boolean;
  }>;
}
interface GitRemote {
  connected: boolean;
  url?: string;
  name?: string;
  defaultBranch?: string;
}
```

## Projects

| Command                   | Args                                       | Return                                                           |
| ------------------------- | ------------------------------------------ | ---------------------------------------------------------------- | ------------ |
| `projects_list`           | `{ includeHidden: boolean }`               | `Project[]`                                                      |
| `projects_get`            | `{ projectId: string }`                    | `{ project: Project; readme?: string; packageManager?: string }` |
| `projects_add_folder`     | `{ path: string }`                         | `Project`                                                        |
| `projects_set_hidden`     | `{ projectId: string; hidden: boolean }`   | `void`                                                           |
| `projects_open_folder`    | `{ projectId: string }`                    | `void`                                                           |
| `projects_open_editor`    | `{ projectId: string }`                    | `void`                                                           |
| `projects_open_terminal`  | `{ projectId: string }`                    | `void`                                                           |
| `projects_scripts`        | `{ projectId: string }`                    | `ProjectScript[]`                                                |
| `projects_run_script`     | `{ projectId: string; scriptId: string }`  | `{ id: string; status: "running" \| "stopped" }` |
| `projects_stop_script`    | `{ projectId: string; processId: string }` | `void`                                                           |
| `projects_agents_summary` | `{ projectId: string }`                    | `AgentsSummary`                                                  |

## Git and GitHub

| Command              | Args                                                      | Return                                                                                      |
| -------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `git_status`         | `{ projectId: string }`                                   | `GitStatus`                                                                                 |
| `git_branches`       | `{ projectId: string }`                                   | `{ current: string; branches: Array<{ name: string; remote?: string; current: boolean }> }` |
| `git_diff`           | `{ projectId: string; path: string; staged: boolean; unstaged: boolean; untracked: boolean }` | `string` |
| `git_stage`          | `{ projectId: string; paths: string[] }`                  | `void`                                                                                      |
| `git_unstage`        | `{ projectId: string; paths: string[] }`                  | `void`                                                                                      |
| `git_commit`         | `{ projectId: string; message: string }`                  | `{ hash: string; message: string }`                                                         |
| `git_checkout`       | `{ projectId: string; branch: string }`                   | `void`                                                                                      |
| `git_remote`         | `{ projectId: string }`                                   | `GitRemote`                                                                                 |
| `github_open`        | `{ projectId: string }`                                   | `void`                                                                                      |
| `github_create_repo` | `{ projectId: string; name: string; isPrivate: boolean }` | `GitRemote`                                                                                 |
| `github_link_push`   | `{ projectId: string; remoteUrl: string }`                | `GitRemote`                                                                                 |
| `github_auth_status` | `{}`                                                       | `{ authenticated: boolean; account?: string; host?: string; error?: string }`               |
| `github_repo_list`   | `{ owner?: string; query?: string; visibility?: string; sort?: string }` | `GitHubRepo[]` |

## Project-scoped MCP and Skills

The catalog remains global. These commands read and write only the selected
project projection; `client` is the per-project enablement target.

| Command                     | Args                                                                           | Return                      |
| --------------------------- | ------------------------------------------------------------------------------ | --------------------------- |
| `mcp_project_list`          | `{ projectId: string }`                                                        | `Record<string, McpServer>` |
| `mcp_project_toggle`        | `{ projectId: string; serverId: string; client: Client; enabled: boolean }`    | `void`                      |
| `mcp_project_bulk_toggle`   | `{ projectId: string; serverIds: string[]; client: Client; enabled: boolean }` | `void`                      |
| `mcp_project_resync`        | `{ projectId: string }`                                                        | `void`                      |
| `skill_project_list`        | `{ projectId: string }`                                                        | `InstalledSkill[]`          |
| `skill_project_toggle`      | `{ projectId: string; skillId: string; client: Client; enabled: boolean }`     | `void`                      |
| `skill_project_bulk_toggle` | `{ projectId: string; skillIds: string[]; client: Client; enabled: boolean }`  | `void`                      |
| `skill_project_resync`      | `{ projectId: string }`                                                        | `void`                      |

## Video

```ts
interface AnalyzeOptions {
  model: string;
  thinking: "low" | "medium" | "high";
  mediaResolution: "low" | "medium" | "high";
  fps: number;
  agentic: boolean;
}
interface VideoEntry {
  id: string;
  title: string;
  duration: string;
  clipCount: number;
  thumbnail?: string;
  path?: string;
}
interface VideoClip {
  id: string;
  title: string;
  start: string;
  duration: string;
}
```

| Command             | Args                                                             | Return                                                                                             |
| ------------------- | ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------- | ------------- |
| `video_library`     | `{ folder: string \| null }`                                    | `{ folder: string; entries: VideoEntry[] }` |
| `video_clips`       | `{ videoId: string }`                                            | `VideoClip[]`                                                                                      |
| `video_open_folder` | `{ path: string }`                                               | `void`                                                                                             |
| `video_analyze`     | `{ url: string; chaptersJson: string; options: AnalyzeOptions }` | `{ title: string; chapters: Array<{ title: string; start: string; end: string }>; log: string[] }` |
| `video_download`    | `{ url: string; chaptersJson: string; options: AnalyzeOptions }` | `{ id: string; status: "queued" \| "running" \| "complete" }` |
| `video_stop`        | `{ jobId: string }`                                              | `void`                                                                                             |
