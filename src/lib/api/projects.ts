import {
  invokeWithDevFallback,
  DEV_PROJECTS,
  devAgentsFor,
  devDetailsFor,
  devScriptsFor,
} from "./devMocks";

export type ProjectKind = "git" | "folder" | "workspace";

export interface ProjectScript {
  id: string;
  name: string;
  command: string;
  running: boolean;
}

export interface ProjectAgentClientSummary {
  client: string;
  mcpServers: number;
  skills: number;
}

export interface AgentsSummary {
  mcpServers: number;
  skills: number;
  clients: ProjectAgentClientSummary[];
}

export interface ProjectRemote {
  connected: boolean;
  url?: string;
  name?: string;
}

export interface Project {
  id: string;
  name: string;
  path: string;
  kind: ProjectKind;
  frameworks: string[];
  branch: string;
  lastCommit: string;
  lastCommitAt: string;
  changedFiles: number;
  hidden: boolean;
  remote: ProjectRemote;
  scripts: ProjectScript[];
  agents: AgentsSummary;
}

export interface ProjectDetails {
  project: Project;
  readme?: string;
  packageManager?: string;
}

export interface ProjectProcess {
  id: string;
  status: "running" | "stopped";
}

const projectArgs = (projectId: string) => ({ projectId });

export const projectsApi = {
  list(includeHidden = false) {
    return invokeWithDevFallback<Project[]>(
      "projects_list",
      { includeHidden },
      () =>
        includeHidden
          ? DEV_PROJECTS
          : DEV_PROJECTS.filter((project) => !project.hidden),
    );
  },
  get(projectId: string) {
    return invokeWithDevFallback<ProjectDetails>(
      "projects_get",
      projectArgs(projectId),
      () => devDetailsFor(projectId),
    );
  },
  addFolder(path: string) {
    return invokeWithDevFallback<Project>(
      "projects_add_folder",
      { path },
      () => ({
        ...DEV_PROJECTS[0],
        id: path,
        path,
        name: path.split("/").pop() || path,
        kind: "folder",
        remote: { connected: false },
      }),
    );
  },
  setHidden(projectId: string, hidden: boolean) {
    return invokeWithDevFallback<void>(
      "projects_set_hidden",
      { projectId, hidden },
      () => undefined,
    );
  },
  openFolder(projectId: string) {
    return invokeWithDevFallback<void>(
      "projects_open_folder",
      projectArgs(projectId),
      () => undefined,
    );
  },
  openEditor(projectId: string) {
    return invokeWithDevFallback<void>(
      "projects_open_editor",
      projectArgs(projectId),
      () => undefined,
    );
  },
  openTerminal(projectId: string) {
    return invokeWithDevFallback<void>(
      "projects_open_terminal",
      projectArgs(projectId),
      () => undefined,
    );
  },
  scripts(projectId: string) {
    return invokeWithDevFallback<ProjectScript[]>(
      "projects_scripts",
      projectArgs(projectId),
      () => devScriptsFor(projectId),
    );
  },
  runScript(projectId: string, scriptId: string) {
    return invokeWithDevFallback<ProjectProcess>(
      "projects_run_script",
      { projectId, scriptId },
      () => ({ id: `${projectId}:${scriptId}`, status: "running" }),
    );
  },
  stopScript(projectId: string, processId: string) {
    return invokeWithDevFallback<void>(
      "projects_stop_script",
      { projectId, processId },
      () => undefined,
    );
  },
  agentsSummary(projectId: string) {
    return invokeWithDevFallback<AgentsSummary>(
      "projects_agents_summary",
      projectArgs(projectId),
      () => devAgentsFor(projectId),
    );
  },
};
