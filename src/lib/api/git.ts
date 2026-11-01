import {
  invokeWithDevFallback,
} from "./devMocks";

const DEV_EMPTY_STATUS: GitStatus = {
  branch: "",
  ahead: 0,
  behind: 0,
  entries: [],
};
const DEV_EMPTY_BRANCHES: GitBranches = { current: "", branches: [] };
const DEV_EMPTY_REMOTE: GitRemote = { connected: false };

export interface GitStatusEntry {
  path: string;
  status: "new" | "modified" | "deleted" | "renamed";
  staged: boolean;
}

export interface GitStatus {
  branch: string;
  ahead: number;
  behind: number;
  entries: GitStatusEntry[];
}

export interface GitBranch {
  name: string;
  remote?: string;
  current: boolean;
}

export interface GitBranches {
  current: string;
  branches: GitBranch[];
}

export interface GitRemote {
  connected: boolean;
  url?: string;
  name?: string;
  defaultBranch?: string;
}

export interface GitCommitResult {
  hash: string;
  message: string;
}

export const gitApi = {
  status(projectId: string) {
    return invokeWithDevFallback<GitStatus>(
      "git_status",
      { projectId },
      () => DEV_EMPTY_STATUS,
    );
  },
  branches(projectId: string) {
    return invokeWithDevFallback<GitBranches>(
      "git_branches",
      { projectId },
      () => DEV_EMPTY_BRANCHES,
    );
  },
  stage(projectId: string, paths: string[]) {
    return invokeWithDevFallback<void>(
      "git_stage",
      { projectId, paths },
      () => undefined,
    );
  },
  unstage(projectId: string, paths: string[]) {
    return invokeWithDevFallback<void>(
      "git_unstage",
      { projectId, paths },
      () => undefined,
    );
  },
  diff(
    projectId: string,
    path: string,
    options: { staged?: boolean; unstaged?: boolean; untracked?: boolean } = {},
  ) {
    return invokeWithDevFallback<string>(
      "git_diff",
      {
        projectId,
        path,
        staged: options.staged ?? false,
        unstaged: options.unstaged ?? true,
        untracked: options.untracked ?? false,
      },
      () => "",
    );
  },
  commit(projectId: string, message: string) {
    return invokeWithDevFallback<GitCommitResult>(
      "git_commit",
      { projectId, message },
      () => ({ hash: "dev0000", message }),
    );
  },
  checkout(projectId: string, branch: string) {
    return invokeWithDevFallback<void>(
      "git_checkout",
      { projectId, branch },
      () => undefined,
    );
  },
  remote(projectId: string) {
    return invokeWithDevFallback<GitRemote>(
      "git_remote",
      { projectId },
      () => DEV_EMPTY_REMOTE,
    );
  },
  openGithub(projectId: string) {
    return invokeWithDevFallback<void>(
      "github_open",
      { projectId },
      () => undefined,
    );
  },
  createGithubRepo(projectId: string, name: string, isPrivate: boolean) {
    return invokeWithDevFallback<GitRemote>(
      "github_create_repo",
      { projectId, name, isPrivate },
      () => ({
        ...DEV_EMPTY_REMOTE,
        name,
        url: `https://github.com/example/${name}`,
      }),
    );
  },
  linkAndPush(projectId: string, remoteUrl: string) {
    return invokeWithDevFallback<GitRemote>(
      "github_link_push",
      { projectId, remoteUrl },
      () => ({
        ...DEV_EMPTY_REMOTE,
        url: remoteUrl,
        name: remoteUrl.split("/").slice(-2).join("/"),
      }),
    );
  },
};
