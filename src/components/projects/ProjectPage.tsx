import { useMemo, useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Check,
  CircleDot,
  Code2,
  ExternalLink,
  FolderOpen,
  GitBranch,
  GitCommitHorizontal,
  Github,
  Hammer,
  Play,
  Square,
  Terminal,
} from "lucide-react";
import type { Project } from "@/lib/api/projects";
import { projectsApi } from "@/lib/api/projects";
import {
  gitApi,
  type GitBranches,
  type GitRemote,
  type GitStatus,
} from "@/lib/api/git";
import { AppPageHeader } from "@/components/shell/AppPageHeader";
import { ProjectSwitcher } from "./ProjectSwitcher";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { PageTabs } from "@/components/ui/page-tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Badge } from "@/components/ui/badge";
import { Notice } from "@/components/ui/notice";
import { Switch } from "@/components/ui/switch";
import { HoverTip } from "@/components/ui/hover-tip";
import { toast } from "@/lib/toast";
import { cn } from "@/lib/utils";

type ProjectTab = "overview" | "scripts" | "git" | "github";

export function ProjectPage({
  projects,
  project,
  onProjectChange,
}: {
  projects: Project[];
  project: Project;
  onProjectChange: (projectId: string | null) => void;
}) {
  const queryClient = useQueryClient();
  const [tab, setTab] = useState<ProjectTab>("overview");
  const [commitMessage, setCommitMessage] = useState("");
  const [confirm, setConfirm] = useState<"create" | "link" | null>(null);
  const [repoName, setRepoName] = useState(project.name);
  const [remoteUrl, setRemoteUrl] = useState("");
  const details = useQuery({
    queryKey: ["project", project.id],
    queryFn: () => projectsApi.get(project.id),
  });
  const status = useQuery({
    queryKey: ["git", "status", project.id],
    queryFn: () => gitApi.status(project.id),
    enabled: project.kind === "git",
  });
  const branches = useQuery({
    queryKey: ["git", "branches", project.id],
    queryFn: () => gitApi.branches(project.id),
    enabled: project.kind === "git",
  });
  const remote = useQuery({
    queryKey: ["git", "remote", project.id],
    queryFn: () => gitApi.remote(project.id),
    enabled: project.kind === "git",
  });
  const scripts = useQuery({
    queryKey: ["project", "scripts", project.id],
    queryFn: () => projectsApi.scripts(project.id),
  });
  const agents = useQuery({
    queryKey: ["project", "agents", project.id],
    queryFn: () => projectsApi.agentsSummary(project.id),
  });
  const commit = useMutation({
    mutationFn: () => gitApi.commit(project.id, commitMessage.trim()),
    onSuccess: () => {
      setCommitMessage("");
      void queryClient.invalidateQueries({
        queryKey: ["git", "status", project.id],
      });
      toast.success("Commit created");
    },
  });
  const currentStatus = status.data;
  const staged = useMemo(
    () => currentStatus?.entries.filter((entry) => entry.staged) ?? [],
    [currentStatus],
  );
  const unstaged = useMemo(
    () => currentStatus?.entries.filter((entry) => !entry.staged) ?? [],
    [currentStatus],
  );
  const runAction = async (action: () => Promise<unknown>, message: string) => {
    try {
      await action();
      toast.success(message);
    } catch (error) {
      toast.error(String(error));
    }
  };

  return (
    <>
      <AppPageHeader
        icon={<FolderOpen className="h-5 w-5" strokeWidth={1.5} />}
        title={project.name}
        subtitle={
          <ProjectSwitcher
            projects={projects}
            value={project.id}
            onChange={onProjectChange}
            className="ms-2"
          />
        }
        actions={
          <>
            <HoverTip content="Open folder">
              <Button
                variant="quiet"
                size="icon-compact"
                aria-label="Open folder"
                onClick={() =>
                  void runAction(
                    () => projectsApi.openFolder(project.id),
                    "Folder opened",
                  )
                }
              >
                <FolderOpen className="h-4 w-4" />
              </Button>
            </HoverTip>
            <HoverTip content="Open in editor">
              <Button
                variant="quiet"
                size="icon-compact"
                aria-label="Open in editor"
                onClick={() =>
                  void runAction(
                    () => projectsApi.openEditor(project.id),
                    "Editor opened",
                  )
                }
              >
                <Code2 className="h-4 w-4" />
              </Button>
            </HoverTip>
            <HoverTip content="Terminal">
              <Button
                variant="quiet"
                size="icon-compact"
                aria-label="Terminal"
                onClick={() =>
                  void runAction(
                    () => projectsApi.openTerminal(project.id),
                    "Terminal opened",
                  )
                }
              >
                <Terminal className="h-4 w-4" />
              </Button>
            </HoverTip>
          </>
        }
      />
      <div className="shrink-0 px-6 pt-2">
        <PageTabs<ProjectTab>
          aria-label="Project sections"
          value={tab}
          onValueChange={setTab}
          items={[
            { value: "overview", label: "Overview" },
            { value: "scripts", label: "Scripts" },
            { value: "git", label: "Git" },
            { value: "github", label: "GitHub" },
          ]}
        />
      </div>
      <main
        id="main-content"
        className="min-h-0 flex-1 overflow-y-auto px-6 pb-12 pt-4"
      >
        <div className="mx-auto max-w-[1080px] space-y-4">
          {tab === "overview" && (
            <Overview
              project={project}
              packageManager={details.data?.packageManager}
              agents={agents.data}
              onAction={runAction}
            />
          )}
          {tab === "scripts" && (
            <Scripts
              projectId={project.id}
              scripts={scripts.data ?? []}
              onAction={runAction}
            />
          )}
          {tab === "git" && (
            <GitPanel
              project={project}
              status={currentStatus}
              branches={branches.data}
              staged={staged}
              unstaged={unstaged}
              commitMessage={commitMessage}
              onCommitMessageChange={setCommitMessage}
              onCommit={() => {
                if (commitMessage.trim()) commit.mutate();
              }}
              onAction={runAction}
            />
          )}
          {tab === "github" && (
            <GithubPanel
              project={project}
              remote={remote.data}
              onOpen={() =>
                void runAction(
                  () => gitApi.openGithub(project.id),
                  "Opening GitHub",
                )
              }
              onCreate={() => setConfirm("create")}
              onLink={() => setConfirm("link")}
            />
          )}
        </div>
      </main>
      <Dialog
        open={confirm !== null}
        onOpenChange={(open) => !open && setConfirm(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {confirm === "create"
                ? "Create GitHub repository"
                : "Link and push remote"}
            </DialogTitle>
            <DialogDescription>
              {confirm === "create"
                ? "Create a repository for this project and push the current branch."
                : "Link this project to a remote and push the current branch."}
            </DialogDescription>
          </DialogHeader>
          {confirm === "create" ? (
            <Input
              aria-label="Repository name"
              value={repoName}
              onChange={(event) => setRepoName(event.target.value)}
              placeholder="Repository name"
            />
          ) : (
            <Input
              aria-label="Remote URL"
              value={remoteUrl}
              onChange={(event) => setRemoteUrl(event.target.value)}
              placeholder="https://github.com/owner/repository"
            />
          )}
          <DialogFooter>
            <Button variant="quiet" onClick={() => setConfirm(null)}>
              Cancel
            </Button>
            <Button
              variant="solid"
              onClick={() => {
                const action =
                  confirm === "create"
                    ? gitApi.createGithubRepo(project.id, repoName, false)
                    : gitApi.linkAndPush(project.id, remoteUrl);
                void runAction(
                  () => action,
                  confirm === "create"
                    ? "Repository created"
                    : "Remote linked and pushed",
                );
                setConfirm(null);
              }}
            >
              Confirm
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

function Overview({
  project,
  packageManager,
  agents,
  onAction,
}: {
  project: Project;
  packageManager?: string;
  agents?: Project["agents"];
  onAction: (action: () => Promise<unknown>, message: string) => void;
}) {
  const summary = agents ?? project.agents;
  return (
    <>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <StatTile
          label="Changed files"
          value={String(project.changedFiles)}
          detail="working tree"
        />
        <StatTile
          label="Last commit"
          value={project.lastCommitAt}
          detail={project.lastCommit}
          mono
        />
        <StatTile
          label="MCP servers"
          value={String(summary.mcpServers)}
          detail="enabled in project"
        />
        <StatTile
          label="Skills"
          value={String(summary.skills)}
          detail="enabled in project"
        />
      </div>
      <div className="grid gap-4 lg:grid-cols-[1.4fr_1fr]">
        <Card>
          <CardHeader className="p-5 pb-3">
            <CardTitle className="text-body font-semibold">
              Workspace overview
            </CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4 p-5 pt-1 sm:grid-cols-2">
            <Info label="Path" value={project.path} mono />
            <Info
              label="Kind"
              value={project.kind === "git" ? "Git repository" : "Folder"}
            />
            <Info label="Frameworks" value={project.frameworks.join(" · ")} />
            <Info
              label="Branch"
              value={project.branch}
              icon={<GitBranch className="h-3.5 w-3.5" />}
            />
            <Info
              label="Last commit"
              value={`${project.lastCommit} · ${project.lastCommitAt}`}
              icon={<GitCommitHorizontal className="h-3.5 w-3.5" />}
            />
            <Info
              label="Changed files"
              value={String(project.changedFiles)}
              icon={<CircleDot className="h-3.5 w-3.5" />}
            />
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="p-5 pb-3">
            <CardTitle className="text-body font-semibold">
              Workspace tools
            </CardTitle>
          </CardHeader>
          <CardContent className="flex flex-wrap gap-2 p-5 pt-1">
            <Button
              variant="neutral"
              onClick={() =>
                onAction(
                  () => projectsApi.openEditor(project.id),
                  "Editor opened",
                )
              }
            >
              <Code2 className="h-4 w-4" />
              Editor
            </Button>
            <Button
              variant="neutral"
              onClick={() =>
                onAction(
                  () => projectsApi.openTerminal(project.id),
                  "Terminal opened",
                )
              }
            >
              <Terminal className="h-4 w-4" />
              Terminal
            </Button>
            <Button
              variant="neutral"
              onClick={() =>
                onAction(
                  () => projectsApi.openFolder(project.id),
                  "Folder opened",
                )
              }
            >
              <FolderOpen className="h-4 w-4" />
              Folder
            </Button>
          </CardContent>
        </Card>
      </div>
      <Card>
        <CardHeader className="p-5 pb-3">
          <CardTitle className="text-body font-semibold">
            Agents in this project
          </CardTitle>
        </CardHeader>
        <CardContent className="p-5 pt-1">
          <div className="mb-4 flex gap-6">
            <Metric value={summary.mcpServers} label="MCP servers" />
            <Metric value={summary.skills} label="Skills enabled" />
          </div>
          <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-5">
            {summary.clients.map((client) => (
              <div
                key={client.client}
                className="rounded-control bg-subtle px-3 py-2"
              >
                <div className="text-caption font-semibold text-fg-1">
                  {client.client}
                </div>
                <div className="mt-1 text-caption text-fg-2">
                  {client.mcpServers} MCP · {client.skills} Skills
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
      <p className="m-0 text-caption text-fg-3">
        {packageManager
          ? `Managed with ${packageManager}.`
          : "Project metadata is loading…"}
      </p>
    </>
  );
}

function Scripts({
  projectId,
  scripts,
  onAction,
}: {
  projectId: string;
  scripts: Project["scripts"];
  onAction: (action: () => Promise<unknown>, message: string) => void;
}) {
  const queryClient = useQueryClient();
  const [processIds, setProcessIds] = useState<Record<string, string>>({});

  const runOrStop = (script: Project["scripts"][number]) => {
    const action = async () => {
      if (script.running) {
        await projectsApi.stopScript(
          projectId,
          processIds[script.id] ?? script.id,
        );
        setProcessIds((current) => {
          const next = { ...current };
          delete next[script.id];
          return next;
        });
      } else {
        const process = await projectsApi.runScript(projectId, script.id);
        setProcessIds((current) => ({ ...current, [script.id]: process.id }));
      }
      await queryClient.invalidateQueries({
        queryKey: ["project", "scripts", projectId],
      });
    };

    onAction(
      action,
      script.running ? `${script.name} stopped` : `${script.name} started`,
    );
  };

  return (
    <Card>
      <CardHeader className="p-5 pb-3">
        <CardTitle className="text-body font-semibold">Scripts</CardTitle>
      </CardHeader>
      <CardContent className="p-0">
        {scripts.map((script) => (
          <div
            key={script.id}
            className="flex items-center gap-3 border-t border-border px-5 py-3"
          >
            <div className="flex h-7 w-7 items-center justify-center rounded-control bg-subtle text-fg-2">
              <Hammer className="h-3.5 w-3.5" />
            </div>
            <div className="min-w-0 flex-1">
              <div className="text-body font-medium">{script.name}</div>
              <code className="text-caption text-fg-3">{script.command}</code>
            </div>
            <Badge
              variant="outline"
              className={cn(
                "border-0 bg-subtle text-caption",
                script.running ? "text-success-text" : "text-fg-3",
              )}
            >
              {script.running ? "Running" : "Ready"}
            </Badge>
            <Button
              variant="quiet"
              size="icon-compact"
              aria-label={
                script.running ? `Stop ${script.name}` : `Run ${script.name}`
              }
              onClick={() => runOrStop(script)}
            >
              {script.running ? (
                <Square className="h-3.5 w-3.5" />
              ) : (
                <Play className="h-3.5 w-3.5" />
              )}
            </Button>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}

function GitPanel({
  project,
  status,
  branches,
  staged,
  unstaged,
  commitMessage,
  onCommitMessageChange,
  onCommit,
  onAction,
}: {
  project: Project;
  status?: GitStatus;
  branches?: GitBranches;
  staged: GitStatus["entries"];
  unstaged: GitStatus["entries"];
  commitMessage: string;
  onCommitMessageChange: (value: string) => void;
  onCommit: () => void;
  onAction: (action: () => Promise<unknown>, message: string) => void;
}) {
  return (
    <div className="grid gap-4 lg:grid-cols-[1.4fr_1fr]">
      <Card>
        <CardHeader className="flex-row items-center justify-between p-5 pb-3">
          <CardTitle className="text-body font-semibold">
            Changes{" "}
            <span className="ms-1 text-caption font-normal text-fg-3">
              {status?.entries.length ?? 0} files
            </span>
          </CardTitle>
          <Badge variant="outline" className="border-0 bg-subtle text-caption">
            {status?.branch ?? project.branch}
          </Badge>
        </CardHeader>
        <CardContent className="p-0">
          <ChangeGroup
            label="Staged"
            entries={staged}
            onAction={(paths) =>
              onAction(
                () => gitApi.unstage(project.id, paths),
                "Changes unstaged",
              )
            }
            onToggle={(path, next) =>
              onAction(
                () =>
                  next
                    ? gitApi.stage(project.id, [path])
                    : gitApi.unstage(project.id, [path]),
                next ? "Change staged" : "Change unstaged",
              )
            }
          />
          <ChangeGroup
            label="Unstaged"
            entries={unstaged}
            onAction={(paths) =>
              onAction(() => gitApi.stage(project.id, paths), "Changes staged")
            }
            onToggle={(path, next) =>
              onAction(
                () =>
                  next
                    ? gitApi.stage(project.id, [path])
                    : gitApi.unstage(project.id, [path]),
                next ? "Change staged" : "Change unstaged",
              )
            }
          />
        </CardContent>
      </Card>
      <div className="space-y-4">
        <Card>
          <CardHeader className="p-5 pb-3">
            <CardTitle className="text-body font-semibold">Commit</CardTitle>
          </CardHeader>
          <CardContent className="flex gap-2 p-5 pt-1">
            <Input
              value={commitMessage}
              onChange={(event) => onCommitMessageChange(event.target.value)}
              placeholder="Commit message"
              className="min-w-0 flex-1"
            />
            <Button
              variant="solid"
              disabled={!commitMessage.trim()}
              onClick={onCommit}
            >
              <GitCommitHorizontal className="h-4 w-4" />
              Commit changes
            </Button>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="p-5 pb-3">
            <CardTitle className="text-body font-semibold">Branches</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1 p-5 pt-1">
            {(branches?.branches ?? []).map((branch) => (
              <button
                key={branch.name}
                type="button"
                onClick={() =>
                  onAction(
                    () => gitApi.checkout(project.id, branch.name),
                    `Checked out ${branch.name}`,
                  )
                }
                className={cn(
                  "flex w-full items-center gap-2 rounded-control px-2 py-2 text-start text-caption hover:bg-subtle",
                  branch.current && "bg-selected font-medium text-fg-1",
                )}
              >
                <GitBranch className="h-3.5 w-3.5 text-fg-3" />
                {branch.name}
                {branch.current && <Check className="ms-auto h-3.5 w-3.5" />}
              </button>
            ))}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function ChangeGroup({
  label,
  entries,
  onAction,
  onToggle,
}: {
  label: string;
  entries: GitStatus["entries"];
  onAction: (paths: string[]) => void;
  onToggle: (path: string, staged: boolean) => void;
}) {
  return (
    <div className="border-t border-border first:border-t-0">
      <div className="flex items-center justify-between px-5 py-2 text-caption font-semibold text-fg-2">
        <span>
          {label}{" "}
          <span className="font-normal text-fg-3">{entries.length}</span>
        </span>
        {entries.length > 0 && (
          <Button
            variant="quiet"
            size="compact"
            onClick={() => onAction(entries.map((entry) => entry.path))}
          >
            {label === "Staged" ? "Unstage all" : "Stage all"}
          </Button>
        )}
      </div>
      {entries.length === 0 ? (
        <p className="m-0 px-5 pb-4 text-caption text-fg-3">
          No {label.toLowerCase()} changes.
        </p>
      ) : (
        entries.map((entry) => (
          <div
            key={entry.path}
            className="flex items-center gap-3 border-t border-border px-5 py-2.5 text-caption"
          >
            <Switch
              size="sm"
              checked={entry.staged}
              aria-label={`Stage ${entry.path}`}
              onCheckedChange={(checked) => onToggle(entry.path, checked)}
            />
            <span
              className={cn(
                "w-5 text-center font-mono font-semibold",
                entry.status === "new"
                  ? "text-success-text"
                  : "text-warning-text",
              )}
            >
              {entry.status === "new" ? "A" : "M"}
            </span>
            <span className="truncate text-fg-1">{entry.path}</span>
          </div>
        ))
      )}
    </div>
  );
}

function GithubPanel({
  project,
  remote,
  onOpen,
  onCreate,
  onLink,
}: {
  project: Project;
  remote?: GitRemote;
  onOpen: () => void;
  onCreate: () => void;
  onLink: () => void;
}) {
  return (
    <Card>
      <CardHeader className="p-5 pb-3">
        <CardTitle className="flex items-center gap-2 text-body font-semibold">
          <Github className="h-4 w-4" />
          GitHub
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4 p-5 pt-1">
        <Notice
          tone={remote?.connected ? "neutral" : "warning"}
          title={
            remote?.connected ? `${remote.name} · private` : "No GitHub remote"
          }
          actions={
            remote?.connected ? (
              <Button variant="neutral" size="compact" onClick={onOpen}>
                <ExternalLink className="h-3.5 w-3.5" />
                Open on GitHub
              </Button>
            ) : (
              <>
                <Button variant="neutral" size="compact" onClick={onLink}>
                  Link &amp; push
                </Button>
                <Button variant="solid" size="compact" onClick={onCreate}>
                  Create repo
                </Button>
              </>
            )
          }
        >
          {remote?.connected
            ? `Up to date with ${remote.defaultBranch ?? project.branch}`
            : "Create a repo for this folder, or link an existing one and push."}
        </Notice>
      </CardContent>
    </Card>
  );
}

function Info({
  label,
  value,
  mono,
  icon,
}: {
  label: string;
  value: string;
  mono?: boolean;
  icon?: ReactNode;
}) {
  return (
    <div className="min-w-0">
      <div className="mb-1 text-caption text-fg-3">{label}</div>
      <div
        className={cn(
          "flex min-w-0 items-center gap-1.5 truncate text-body text-fg-1",
          mono && "font-mono text-caption",
        )}
      >
        {icon}
        {value}
      </div>
    </div>
  );
}
function Metric({ value, label }: { value: number; label: string }) {
  return (
    <div>
      <div className="text-[26px] font-semibold leading-none text-fg-1">
        {value}
      </div>
      <div className="mt-1 text-caption text-fg-2">{label}</div>
    </div>
  );
}

function StatTile({
  label,
  value,
  detail,
  mono = false,
}: {
  label: string;
  value: string;
  detail: string;
  mono?: boolean;
}) {
  return (
    <Card className="p-4">
      <div className="text-caption text-fg-3">{label}</div>
      <div
        className={cn(
          "mt-1 text-section font-semibold",
          mono && "font-mono text-body",
        )}
      >
        {value}
      </div>
      <div className="mt-1 truncate text-caption text-fg-2">{detail}</div>
    </Card>
  );
}
