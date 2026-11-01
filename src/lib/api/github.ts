import { invokeWithDevFallback } from "./devMocks";

export interface GitHubAuthStatus {
  authenticated: boolean;
  account?: string;
  host?: string;
  error?: string;
}

export interface GitHubRepo {
  name: string;
  nameWithOwner: string;
  description: string;
  isPrivate: boolean;
  isArchived: boolean;
  url: string;
  stargazerCount: number;
  updatedAt: string;
  pushedAt: string;
  visibility: string;
  primaryLanguage: string;
}

export type GitHubVisibility = "all" | "public" | "private" | "archived";
export type GitHubSort = "updated" | "pushed" | "name" | "stars";

export const githubApi = {
  authStatus() {
    return invokeWithDevFallback<GitHubAuthStatus>(
      "github_auth_status",
      undefined,
      () => ({ authenticated: false }),
    );
  },
  repoList(options: {
    owner?: string;
    query?: string;
    visibility?: GitHubVisibility;
    sort?: GitHubSort;
  } = {}) {
    return invokeWithDevFallback<GitHubRepo[]>(
      "github_repo_list",
      options,
      () => [],
    );
  },
};
