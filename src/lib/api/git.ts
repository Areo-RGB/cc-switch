import {
  invokeWithDevFallback,
  DEV_GIT_BRANCHES,
  DEV_GIT_REMOTE,
  DEV_GIT_STATUS,
} from "./devMocks";

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
      () => DEV_GIT_STATUS,
    );
  },
  branches(projectId: string) {
    return invokeWithDevFallback<GitBranches>(
      "git_branches",
      { projectId },
      () => DEV_GIT_BRANCHES,
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
      () => DEV_GIT_REMOTE,
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
        ...DEV_GIT_REMOTE,
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
        ...DEV_GIT_REMOTE,
        url: remoteUrl,
        name: remoteUrl.split("/").slice(-2).join("/"),
      }),
    );
  },
};
