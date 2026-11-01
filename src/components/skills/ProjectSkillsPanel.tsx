import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { MoreHorizontal, RefreshCw, Sparkles } from "lucide-react";
import type { AppId } from "@/lib/api/types";
import type { InstalledSkill } from "@/lib/api/skills";
import { skillProjectApi } from "@/lib/api/skills";
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
const flagsOf = (skill: InstalledSkill): AppFlags =>
  skill.apps as unknown as AppFlags;

export function ProjectSkillsPanel({
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
    queryKey: ["skills-project", projectId],
    queryFn: () => skillProjectApi.list(projectId ?? ""),
    enabled: Boolean(projectId),
  });
  const [skills, setSkills] = useState<InstalledSkill[]>([]);
  const [search, setSearch] = useState("");
  const [pending, setPending] = useState<Record<string, boolean>>({});

  useEffect(() => {
    if (query.data) {
      setSkills(query.data);
      setPending({});
    }
  }, [query.data]);

  const filtered = useMemo(
    () =>
      skills.filter((skill) =>
        `${skill.name} ${skill.description ?? ""} ${skill.directory}`
          .toLowerCase()
          .includes(search.trim().toLowerCase()),
      ),
    [search, skills],
  );

  const setDraft = (skillId: string, client: AppId, enabled: boolean) => {
    setSkills((current) =>
      current.map((item) =>
        item.id === skillId
          ? { ...item, apps: { ...item.apps, [client]: enabled } }
          : item,
      ),
    );
    setPending((current) => ({
      ...current,
      [`${skillId}:${client}`]: enabled,
    }));
  };

  const apply = async () => {
    try {
      await Promise.all(
        Object.entries(pending).map(([key, enabled]) => {
          const [skillId, client] = key.split(":") as [string, AppId];
          return skillProjectApi.toggle(
            projectId ?? "",
            skillId,
            client,
            enabled,
          );
        }),
      );
      setPending({});
      toast.success(`Applied Skills changes to ${project?.name ?? "project"}`);
    } catch (error) {
      toast.error(String(error));
    }
  };

  const discard = () => {
    if (query.data) setSkills(query.data);
    setPending({});
    toast.success("Skills changes discarded");
  };

  const resync = async () => {
    try {
      await skillProjectApi.resync(projectId ?? "");
      toast.success("Project Skills configuration synced");
    } catch (error) {
      toast.error(String(error));
    }
  };

  return (
    <>
      <AppPageHeader
        icon={<Sparkles className="h-5 w-5" strokeWidth={1.5} />}
        title="Skills"
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
                aria-label="More Skills actions"
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
              clearLabel="Clear Skills search"
              placeholder="Search skills"
              containerClassName="w-[320px] max-w-full"
            />
            <Badge
              variant="outline"
              className="border-0 bg-subtle text-caption"
            >
              {filtered.length} installed
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
                Project skill files will be updated when you apply.
              </Notice>
            )}
            {filtered.map((skill) => (
              <SkillCard key={skill.id} skill={skill} onToggle={setDraft} />
            ))}
            {filtered.length === 0 && (
              <p className="m-0 rounded-panel border border-border px-5 py-10 text-center text-body text-fg-2">
                No Skills match this search.
              </p>
            )}
          </div>
        </main>
      )}
    </>
  );
}

function SkillCard({
  skill,
  onToggle,
}: {
  skill: InstalledSkill;
  onToggle: (skillId: string, client: AppId, enabled: boolean) => void;
}) {
  const apps = flagsOf(skill);
  const active = CLIENTS.some((client) => Boolean(apps[client]));
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
        {skill.name.slice(0, 1).toUpperCase()}
      </div>
      <div className="min-w-[180px] flex-1">
        <div className="text-body font-medium">{skill.name}</div>
        <div className="truncate text-caption text-fg-2">
          {skill.description ?? skill.directory}
        </div>
        {skill.repoOwner && (
          <div className="mt-1 text-[10px] text-fg-3">
            {skill.repoOwner}/{skill.repoName}
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
              aria-label={`${skill.name} · ${APP_DISPLAY_NAME[client]}`}
              onCheckedChange={(checked) => onToggle(skill.id, client, checked)}
            />
          </label>
        ))}
      </div>
      <Button
        variant="quiet"
        size="icon-compact"
        aria-label={`More actions for ${skill.name}`}
      >
        <MoreHorizontal className="h-4 w-4" />
      </Button>
    </div>
  );
}

function pendingCount(pending: Record<string, boolean>) {
  return Object.keys(pending).length;
}
