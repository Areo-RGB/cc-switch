import { useMemo, useRef, useState } from "react";
import {
  BookOpen,
  ChevronsLeft,
  ChevronsRight,
  Eye,
  EyeOff,
  Folder,
  FolderGit2,
  FolderPlus,
  History,
  MoreHorizontal,
  PlaySquare,
  Server,
  Settings,
  Sparkles,
} from "lucide-react";
import type { GlobalPage, SettingsSection, View } from "@/lib/navigation";
import type { Project } from "@/lib/api/projects";
import { SearchField } from "@/components/ui/search-field";
import { HoverTip } from "@/components/ui/hover-tip";
import { cn } from "@/lib/utils";
import { sidebarWidth, useSidebarCollapsed } from "@/hooks/useSidebarCollapsed";
import ccswitchLogo from "@/assets/icons/logo.svg";

type IconComponent = typeof Server;

interface SidebarProps {
  projects?: Project[];
  activeProjectId?: string | null;
  view?: View;
  settingsSection?: SettingsSection;
  onSelectProject?: (project: Project) => void;
  onAddFolder?: () => void;
  onToggleHidden?: (project: Project) => void;
  onSelectPage?: (page: GlobalPage | "projects" | "settings") => void;
  activeApp?: string;
  visibleApps?: unknown;
  onSelectApp?: (app: string) => void;
  onSelectSettingsSection?: (section: SettingsSection) => void;
  onExitSettings?: () => void;
}

const globalItems: Array<{
  page: GlobalPage;
  label: string;
  icon: IconComponent;
}> = [
  { page: "mcp", label: "MCP", icon: Server },
  { page: "skills", label: "Skills", icon: Sparkles },
  { page: "prompts", label: "Prompts", icon: BookOpen },
  { page: "sessions", label: "Sessions", icon: History },
  { page: "video", label: "Video", icon: PlaySquare },
];

export function Sidebar({
  projects = [],
  activeProjectId = null,
  view = "projects",
  settingsSection: _settingsSection,
  onSelectProject = () => undefined,
  onAddFolder = () => undefined,
  onToggleHidden = () => undefined,
  onSelectPage = () => undefined,
}: SidebarProps) {
  const navRef = useRef<HTMLElement>(null);
  const { collapsed, toggle } = useSidebarCollapsed(navRef);
  const [filter, setFilter] = useState("");
  const [showHidden, setShowHidden] = useState(false);
  const filteredProjects = useMemo(() => {
    const query = filter.trim().toLowerCase();
    return projects.filter((project) => {
      if (!showHidden && project.hidden) return false;
      return (
        !query ||
        `${project.name} ${project.path}`.toLowerCase().includes(query)
      );
    });
  }, [filter, projects, showHidden]);

  return (
    <nav
      ref={navRef}
      aria-label="Main navigation"
      className="relative flex h-full shrink-0 flex-col overflow-hidden whitespace-nowrap border-e border-border bg-sidebar text-body text-fg-1"
      style={{ width: sidebarWidth(collapsed) }}
    >
      <div
        className="flex h-11 shrink-0 items-center gap-2 px-3"
        data-tauri-drag-region
      >
        <img src={ccswitchLogo} alt="" className="h-5 w-5 shrink-0" />
        {!collapsed && (
          <span className="me-auto text-strong font-semibold">CC Switch</span>
        )}
        <HoverTip content={collapsed ? "Expand sidebar" : "Collapse sidebar"}>
          <button
            type="button"
            onClick={toggle}
            aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
            className="flex h-7 w-7 items-center justify-center rounded-control text-fg-2 hover:bg-subtle hover:text-fg-1"
          >
            {collapsed ? (
              <ChevronsRight className="h-4 w-4" />
            ) : (
              <ChevronsLeft className="h-4 w-4" />
            )}
          </button>
        </HoverTip>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
        {!collapsed && (
          <div className="mb-2 px-2 pt-2">
            <div className="mb-2 flex items-center justify-between">
              <button
                type="button"
                onClick={() => onSelectPage("projects")}
                className="text-caption font-semibold uppercase tracking-[0.12em] text-fg-2 hover:text-fg-1"
              >
                Projects
              </button>
              <div className="flex items-center gap-0.5">
                <button
                  type="button"
                  aria-label={
                    showHidden ? "Hide hidden projects" : "Show hidden projects"
                  }
                  onClick={() => setShowHidden((value) => !value)}
                  className="flex h-6 w-6 items-center justify-center rounded-control text-fg-3 hover:bg-subtle hover:text-fg-1"
                >
                  {showHidden ? (
                    <Eye className="h-3.5 w-3.5" />
                  ) : (
                    <EyeOff className="h-3.5 w-3.5" />
                  )}
                </button>
              </div>
            </div>
            <SearchField
              value={filter}
              onValueChange={setFilter}
              clearLabel="Clear project filter"
              placeholder="Filter projects"
              className="h-7 text-caption"
            />
          </div>
        )}
        <div className="flex flex-col gap-0.5">
          {filteredProjects.map((project) => (
            <div key={project.id} className="group relative flex items-center">
              <button
                type="button"
                aria-current={
                  activeProjectId === project.id ? "page" : undefined
                }
                onClick={() => onSelectProject(project)}
                className={cn(
                  "flex h-7 min-w-0 flex-1 items-center gap-2 rounded-control px-2 text-start transition-colors hover:bg-subtle",
                  activeProjectId === project.id && "bg-selected text-fg-1",
                  collapsed && "justify-center px-0",
                )}
              >
                {project.kind === "git" ? (
                  <FolderGit2 className="h-4 w-4 shrink-0 text-fg-2" />
                ) : (
                  <Folder className="h-4 w-4 shrink-0 text-fg-2" />
                )}
                {!collapsed && (
                  <>
                    <span className="min-w-0 truncate text-body">
                      {project.name}
                    </span>
                    <span
                      className={cn(
                        "ms-auto shrink-0 text-caption text-fg-3",
                        project.changedFiles > 0 && "text-warning-text",
                      )}
                    >
                      {project.changedFiles > 0
                        ? `● ${project.changedFiles}`
                        : (project.frameworks[0]?.toLowerCase() ??
                          project.kind)}
                    </span>
                  </>
                )}
              </button>
              {!collapsed && (
                <button
                  type="button"
                  aria-label={
                    project.hidden
                      ? `Unhide ${project.name}`
                      : `Hide ${project.name}`
                  }
                  onClick={() => onToggleHidden(project)}
                  className="absolute end-1 hidden h-6 w-6 items-center justify-center rounded-control text-fg-3 hover:bg-subtle hover:text-fg-1 group-hover:flex"
                >
                  {project.hidden ? (
                    <Eye className="h-3.5 w-3.5" />
                  ) : (
                    <MoreHorizontal className="h-3.5 w-3.5" />
                  )}
                </button>
              )}
            </div>
          ))}
          {filteredProjects.length === 0 && !collapsed && (
            <p className="px-2 py-3 text-caption text-fg-3">
              No projects match.
            </p>
          )}
          {!collapsed && (
            <button
              type="button"
              onClick={onAddFolder}
              className="flex h-7 w-full items-center gap-2 rounded-control px-2 text-start text-body text-fg-3 transition-colors hover:bg-subtle hover:text-fg-1"
            >
              <FolderPlus className="h-4 w-4 shrink-0" />
              Add folder…
            </button>
          )}
        </div>
      </div>

      <div className="border-t border-border px-2 py-2">
        {!collapsed && (
          <div className="px-2 pb-1 text-caption font-semibold uppercase tracking-[0.12em] text-fg-3">
            Tools
          </div>
        )}
        <div className="flex flex-col gap-0.5">
          {globalItems.map(({ page, label, icon: Icon }) => (
            <NavItem
              key={page}
              label={label}
              icon={Icon}
              collapsed={collapsed}
              selected={view === page}
              onClick={() => onSelectPage(page)}
            />
          ))}
        </div>
        <div className="my-2 h-px bg-border" />
        <NavItem
          label="Settings"
          icon={Settings}
          collapsed={collapsed}
          selected={view === "settings"}
          onClick={() => onSelectPage("settings")}
        />
      </div>
    </nav>
  );
}

function NavItem({
  label,
  icon: Icon,
  collapsed,
  selected,
  onClick,
}: {
  label: string;
  icon: IconComponent;
  collapsed: boolean;
  selected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-current={selected ? "page" : undefined}
      onClick={onClick}
      className={cn(
        "flex h-9 w-full items-center gap-2 rounded-control px-3 text-fg-2 transition-colors hover:bg-subtle hover:text-fg-1",
        selected && "bg-selected font-medium text-fg-1",
        collapsed && "justify-center px-0",
      )}
    >
      <Icon className="h-4 w-4 shrink-0" strokeWidth={1.7} />
      {!collapsed && <span>{label}</span>}
    </button>
  );
}
