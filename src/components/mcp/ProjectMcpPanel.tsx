import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { MoreHorizontal, RefreshCw, Server } from "lucide-react";
import type { AppId } from "@/lib/api/types";
import type { McpServer, McpServersMap } from "@/types";
import { mcpProjectApi } from "@/lib/api/mcp";
import type { Project } from "@/lib/api/projects";
import { AppPageHeader } from "@/components/shell/AppPageHeader";
import { ProjectSwitcher } from "@/components/projects/ProjectSwitcher";
import { ProjectEmptyState } from "@/components/projects/ProjectEmptyState";
import { AppGlyph, APP_DISPLAY_NAME } from "@/components/shell/AppGlyph";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Notice } from "@/components/ui/notice";
import { SearchField } from "@/components/ui/search-field";
import { Switch } from "@/components/ui/switch";
import { toast } from "@/lib/toast";
import { cn } from "@/lib/utils";

const CLIENTS: AppId[] = ["claude", "codex", "opencode", "pi", "gemini"];
type AppFlags = Record<string, boolean | undefined>;
const flagsOf = (server: McpServer): AppFlags =>
  server.apps as unknown as AppFlags;

export function ProjectMcpPanel({
  projects,
  projectId,
  onProjectChange,
  onPickProject,
}: {
  projects: Project[];
  projectId: string | null;
  onProjectChange: (projectId: string | null) => void;
  onPickProject: () => void;
}) {
  const project = projects.find((item) => item.id === projectId);
  const query = useQuery({
    queryKey: ["mcp-project", projectId],
    queryFn: () => mcpProjectApi.list(projectId ?? ""),
    enabled: Boolean(projectId),
  });
  const [servers, setServers] = useState<McpServersMap>({});
  const [search, setSearch] = useState("");
  const [pending, setPending] = useState<Record<string, boolean>>({});

  useEffect(() => {
    if (query.data) {
      setServers(query.data);
      setPending({});
    }
  }, [query.data]);

  const entries = useMemo(
    () =>
      Object.entries(servers).filter(([id, server]) =>
        `${id} ${server.name} ${server.description ?? ""}`
          .toLowerCase()
          .includes(search.trim().toLowerCase()),
      ),
    [search, servers],
  );

  const setDraft = (id: string, client: AppId, enabled: boolean) => {
    setServers((current) => ({
      ...current,
      [id]: {
        ...current[id],
        apps: { ...current[id].apps, [client]: enabled },
      },
    }));
    setPending((current) => ({ ...current, [`${id}:${client}`]: enabled }));
  };

  const apply = async () => {
    try {
      await Promise.all(
        Object.entries(pending).map(([key, enabled]) => {
          const [id, client] = key.split(":") as [string, AppId];
          return mcpProjectApi.toggle(projectId ?? "", id, client, enabled);
        }),
      );
      setPending({});
      toast.success(`Applied MCP changes to ${project?.name ?? "project"}`);
    } catch (error) {
      toast.error(String(error));
    }
  };

  const discard = () => {
    if (query.data) setServers(query.data);
    setPending({});
    toast.success("MCP changes discarded");
  };

  const resync = async () => {
    try {
      await mcpProjectApi.resync(projectId ?? "");
      toast.success("Project MCP configuration synced");
    } catch (error) {
      toast.error(String(error));
    }
  };

  return (
    <>
      <AppPageHeader
        icon={<Server className="h-5 w-5" strokeWidth={1.5} />}
        title="MCP"
        subtitle={
          <ProjectSwitcher
            projects={projects}
            value={projectId}
            onChange={onProjectChange}
            className="ms-2"
          />
        }
        actions={
          project ? (
            <>
              <Button
                variant="quiet"
                size="regular"
                onClick={() => void resync()}
              >
                <RefreshCw className="h-4 w-4" />
                Sync project
              </Button>
              <Button
                variant="quiet"
                size="icon-compact"
                aria-label="More MCP actions"
              >
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </>
          ) : undefined
        }
      />
      {!project ? (
        <ProjectEmptyState onChoose={onPickProject} />
      ) : (
        <main id="main-content" className="min-h-0 flex-1 overflow-y-auto">
          <div className="flex flex-wrap items-center gap-3 px-6 py-3">
            <SearchField
              value={search}
              onValueChange={setSearch}
              clearLabel="Clear MCP search"
              placeholder="Search MCP servers"
              containerClassName="w-[320px] max-w-full"
            />
            <Badge
              variant="outline"
              className="border-0 bg-subtle text-caption"
            >
              {entries.length} servers
            </Badge>
          </div>
          <div className="space-y-2 px-6 pb-12">
            {pendingCount(pending) > 0 && (
              <Notice
                title={`${pendingCount(pending)} change${pendingCount(pending) === 1 ? "" : "s"} not written yet`}
                actions={
                  <>
                    <Button variant="neutral" size="compact" onClick={discard}>
                      Discard
                    </Button>
                    <Button
                      variant="solid"
                      size="compact"
                      onClick={() => void apply()}
                    >
                      Apply to project
                    </Button>
                  </>
                }
              >
                Project client files will be updated when you apply.
              </Notice>
            )}
            {entries.map(([id, server]) => (
              <McpCard key={id} id={id} server={server} onToggle={setDraft} />
            ))}
            {entries.length === 0 && (
              <p className="m-0 rounded-panel border border-border px-5 py-10 text-center text-body text-fg-2">
                No MCP servers match this search.
              </p>
            )}
          </div>
        </main>
      )}
    </>
  );
}

function McpCard({
  id,
  server,
  onToggle,
}: {
  id: string;
  server: McpServer;
  onToggle: (id: string, client: AppId, enabled: boolean) => void;
}) {
  const apps = flagsOf(server);
  const active = CLIENTS.some((client) => Boolean(apps[client]));
  const description =
    server.description ??
    (server.server.type === "http"
      ? server.server.url
      : `${server.server.command ?? "command"} ${(server.server.args ?? []).join(" ")}`);

  return (
    <div
      className={cn(
        "flex flex-wrap items-center gap-3 rounded-panel border px-3 py-2.5 transition-colors",
        active
          ? "border-direct-border bg-direct-soft"
          : "border-border bg-surface",
      )}
    >
      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-control bg-subtle text-body font-semibold text-fg-1">
        {(server.name || id).slice(0, 1).toUpperCase()}
      </div>
      <div className="min-w-[180px] flex-1">
        <div className="flex items-center gap-2 text-body font-medium">
          {server.name || id}
          <Badge variant="outline" className="h-[18px] text-[10px]">
            {server.server.type}
          </Badge>
        </div>
        <div className="truncate text-caption text-fg-2">{description}</div>
        {server.tags && (
          <div className="mt-1 flex gap-1">
            {server.tags.map((tag) => (
              <span
                key={tag}
                className="rounded-full bg-subtle px-1.5 py-0.5 text-[10px] text-fg-3"
              >
                {tag}
              </span>
            ))}
          </div>
        )}
      </div>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
        {CLIENTS.map((client) => (
          <label
            key={client}
            className="flex items-center gap-1.5 text-caption text-fg-2"
          >
            <AppGlyph app={client} size={14} />
            <span className="hidden xl:inline">
              {APP_DISPLAY_NAME[client].replace(" CLI", "")}
            </span>
            <Switch
              size="sm"
              checked={Boolean(apps[client])}
              aria-label={`${server.name} · ${APP_DISPLAY_NAME[client]}`}
              onCheckedChange={(checked) => onToggle(id, client, checked)}
            />
          </label>
        ))}
      </div>
      <Button
        variant="quiet"
        size="icon-compact"
        aria-label={`More actions for ${server.name}`}
      >
        <MoreHorizontal className="h-4 w-4" />
      </Button>
    </div>
  );
}

function pendingCount(pending: Record<string, boolean>) {
  return Object.keys(pending).length;
}
