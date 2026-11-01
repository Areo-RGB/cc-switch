import * as TauriCore from "@tauri-apps/api/core";
import type { McpServersMap } from "@/types";
import type { InstalledSkill } from "./skills";
import type {
  AgentsSummary,
  Project,
} from "./projects";
import type { VideoClip, VideoEntry, VideoLibrary } from "./video";

/**
 * Browser-only data used while the Rust command surface is being built.
 * Mutations intentionally do not change this state: the UI shows a toast and
 * the next backend-backed query will become the source of truth.
 */
export async function invokeWithDevFallback<T>(
  command: string,
  args: Record<string, unknown> | undefined,
  fallback: () => T | Promise<T>,
): Promise<T> {
  const tauriRuntime =
    typeof window !== "undefined" &&
    Boolean(
      (window as typeof window & { __TAURI_INTERNALS__?: unknown })
        .__TAURI_INTERNALS__,
    );
  if (!tauriRuntime) return await fallback();
  try {
    return await TauriCore.invoke<T>(command, args);
  } catch (error) {
    if (import.meta.env.DEV) return await fallback();
    throw error;
  }
}

export const DEV_PROJECTS: Project[] = [
  {
    id: "/home/paul/Projects/cc-switch",
    name: "cc-switch",
    path: "/home/paul/Projects/cc-switch",
    kind: "git",
    frameworks: ["React", "TypeScript", "Tauri"],
    branch: "main",
    lastCommit: "Project-first navigation shell",
    lastCommitAt: "18 min ago",
    changedFiles: 7,
    hidden: false,
    remote: {
      connected: true,
      url: "https://github.com/farion1231/cc-switch",
      name: "farion1231/cc-switch",
    },
    scripts: [
      { id: "dev", name: "dev", command: "pnpm dev:renderer", running: false },
      { id: "check", name: "check", command: "pnpm typecheck", running: false },
      { id: "test", name: "test", command: "pnpm test:unit", running: false },
    ],
    agents: {
      mcpServers: 4,
      skills: 6,
      clients: [
        { client: "Claude", mcpServers: 4, skills: 6 },
        { client: "Codex", mcpServers: 3, skills: 4 },
        { client: "OpenCode", mcpServers: 2, skills: 3 },
        { client: "Pi", mcpServers: 1, skills: 2 },
        { client: "Gemini", mcpServers: 2, skills: 3 },
      ],
    },
  },
  {
    id: "/home/paul/Projects/desktopcc",
    name: "desktopcc",
    path: "/home/paul/Projects/desktopcc",
    kind: "git",
    frameworks: ["Rust", "GTK", "Python"],
    branch: "main",
    lastCommit: "Port project setup commands",
    lastCommitAt: "Yesterday",
    changedFiles: 0,
    hidden: false,
    remote: {
      connected: true,
      url: "https://github.com/farion1231/desktopcc",
      name: "farion1231/desktopcc",
    },
    scripts: [
      { id: "run", name: "run", command: "cargo run", running: false },
      { id: "test", name: "test", command: "cargo test", running: false },
    ],
    agents: {
      mcpServers: 2,
      skills: 3,
      clients: [
        { client: "Claude", mcpServers: 2, skills: 3 },
        { client: "Codex", mcpServers: 1, skills: 2 },
      ],
    },
  },
  {
    id: "/home/paul/Projects/recipe-lab",
    name: "recipe-lab",
    path: "/home/paul/Projects/recipe-lab",
    kind: "folder",
    frameworks: ["Next.js", "Tailwind"],
    branch: "—",
    lastCommit: "Not a git repository",
    lastCommitAt: "—",
    changedFiles: 0,
    hidden: false,
    remote: { connected: false },
    scripts: [
      { id: "preview", name: "preview", command: "pnpm dev", running: false },
    ],
    agents: {
      mcpServers: 1,
      skills: 2,
      clients: [{ client: "Claude", mcpServers: 1, skills: 2 }],
    },
  },
];

export const DEV_MCP_SERVERS: McpServersMap = {
  filesystem: {
    id: "filesystem",
    name: "Filesystem",
    description: "Read and write files inside a project workspace.",
    tags: ["local", "files"],
    server: {
      type: "stdio",
      command: "npx",
      args: ["-y", "@modelcontextprotocol/server-filesystem", "."],
    },
    apps: {
      claude: true,
      codex: true,
      gemini: false,
      opencode: true,
      hermes: false,
      pi: false,
      openclaw: false,
    },
  },
  playwright: {
    id: "playwright",
    name: "Playwright",
    description: "Browser automation for local QA and visual checks.",
    tags: ["browser", "qa"],
    server: { type: "stdio", command: "npx", args: ["@playwright/mcp@latest"] },
    apps: {
      claude: true,
      codex: false,
      gemini: true,
      opencode: true,
      hermes: false,
      pi: true,
      openclaw: false,
    },
  },
  github: {
    id: "github",
    name: "GitHub",
    description: "Issues, pull requests, and repository context.",
    tags: ["remote", "github"],
    server: { type: "http", url: "https://api.githubcopilot.com/mcp/" },
    apps: {
      claude: true,
      codex: true,
      gemini: true,
      opencode: false,
      hermes: false,
      pi: false,
      openclaw: false,
    },
  },
  context7: {
    id: "context7",
    name: "Context7",
    description: "Up-to-date library documentation for coding agents.",
    tags: ["docs"],
    server: { type: "http", url: "https://mcp.context7.com/mcp" },
    apps: {
      claude: false,
      codex: true,
      gemini: false,
      opencode: true,
      hermes: false,
      pi: false,
      openclaw: false,
    },
  },
};

export const DEV_SKILLS: InstalledSkill[] = [
  {
    id: "systematic-debugging",
    name: "Systematic Debugging",
    description: "A repeatable workflow for diagnosing unexpected behavior.",
    directory: "systematic-debugging",
    repoOwner: "obra",
    repoName: "superpowers",
    repoBranch: "main",
    apps: {
      claude: true,
      codex: true,
      gemini: false,
      opencode: true,
      openclaw: false,
      hermes: false,
      pi: true,
    },
    installedAt: Date.now() - 1000 * 60 * 60 * 48,
    updatedAt: Date.now() - 1000 * 60 * 60 * 6,
  },
  {
    id: "frontend-design",
    name: "Frontend Design",
    description: "Design and implement polished, production-ready interfaces.",
    directory: "frontend-design",
    repoOwner: "anthropics",
    repoName: "skills",
    repoBranch: "main",
    apps: {
      claude: true,
      codex: true,
      gemini: true,
      opencode: false,
      openclaw: false,
      hermes: false,
      pi: false,
    },
    installedAt: Date.now() - 1000 * 60 * 60 * 24 * 7,
    updatedAt: Date.now() - 1000 * 60 * 60 * 24,
  },
  {
    id: "verification-before-completion",
    name: "Verification Before Completion",
    description: "Evidence-first checks before claiming a task is complete.",
    directory: "verification-before-completion",
    repoOwner: "obra",
    repoName: "superpowers",
    repoBranch: "main",
    apps: {
      claude: true,
      codex: false,
      gemini: false,
      opencode: true,
      openclaw: false,
      hermes: false,
      pi: false,
    },
    installedAt: Date.now() - 1000 * 60 * 60 * 24 * 12,
    updatedAt: Date.now() - 1000 * 60 * 60 * 24 * 2,
  },
];

const devThumbnail = (background: string, accent: string, label: string) =>
  `data:image/svg+xml,${encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 960 540">
      <rect width="960" height="540" fill="${background}"/>
      <path d="M0 430 C180 320 270 480 450 360 S720 280 960 390 V540 H0Z" fill="${accent}" opacity=".28"/>
      <path d="M80 130h800M80 190h620M80 250h720M80 310h420" stroke="${accent}" stroke-width="18" opacity=".75" stroke-linecap="round"/>
      <circle cx="820" cy="130" r="54" fill="${accent}" opacity=".82"/>
      <text x="80" y="465" fill="#fff" font-family="system-ui,sans-serif" font-size="34" font-weight="600">${label}</text>
    </svg>`,
  )}`;

export const DEV_VIDEO_ENTRIES: VideoEntry[] = [
  {
    id: "cc-switch-tour",
    title: "cc-switch project tour",
    duration: "08:42",
    clipCount: 6,
    thumbnail: devThumbnail("#111827", "#38bdf8", "PROJECT TOUR"),
  },
  {
    id: "tauri-workbench",
    title: "Building a Tauri workbench",
    duration: "12:18",
    clipCount: 8,
    thumbnail: devThumbnail("#172554", "#f59e0b", "TAURI WORKBENCH"),
  },
  {
    id: "agent-matrix",
    title: "Agent matrix walkthrough",
    duration: "06:34",
    clipCount: 4,
    thumbnail: devThumbnail("#083344", "#22d3ee", "AGENT MATRIX"),
  },
  {
    id: "git-workflow",
    title: "A calm Git workflow",
    duration: "09:05",
    clipCount: 5,
    thumbnail: devThumbnail("#1f2937", "#a78bfa", "GIT WORKFLOW"),
  },
];

export const DEV_VIDEO_CLIPS: Record<string, VideoClip[]> = {
  "cc-switch-tour": [
    { id: "intro", title: "Welcome", start: "00:00", duration: "01:12" },
    {
      id: "projects",
      title: "Projects sidebar",
      start: "01:12",
      duration: "02:04",
    },
    { id: "agents", title: "Agent setup", start: "03:16", duration: "02:48" },
    {
      id: "github",
      title: "GitHub handoff",
      start: "06:04",
      duration: "01:30",
    },
  ],
  "tauri-workbench": [
    { id: "shell", title: "Shell layout", start: "00:00", duration: "02:10" },
    {
      id: "commands",
      title: "Command bridge",
      start: "02:10",
      duration: "03:28",
    },
    {
      id: "finish",
      title: "Finishing touches",
      start: "05:38",
      duration: "02:04",
    },
  ],
};

export const DEV_VIDEO_LIBRARY: VideoLibrary = {
  folder: "/home/paul/Videos/cc-switch",
  entries: DEV_VIDEO_ENTRIES,
};

export const devAgentsFor = (projectId: string): AgentsSummary =>
  DEV_PROJECTS.find((project) => project.id === projectId)?.agents ??
  DEV_PROJECTS[0].agents;
