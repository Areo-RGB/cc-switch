import { ChevronDown, FolderGit2 } from "lucide-react";
import type { Project } from "@/lib/api/projects";
import { cn } from "@/lib/utils";

export function ProjectSwitcher({
  projects,
  value,
  onChange,
  className,
}: {
  projects: Project[];
  value: string | null;
  onChange: (projectId: string | null) => void;
  className?: string;
}) {
  return (
    <label
      className={cn("relative inline-flex items-center gap-1.5", className)}
    >
      <FolderGit2 className="pointer-events-none absolute start-2 h-3.5 w-3.5 text-fg-3" />
      <select
        aria-label="Current project"
        value={value ?? ""}
        onChange={(event) => onChange(event.target.value || null)}
        className="h-8 min-w-[180px] appearance-none rounded-control border border-border bg-surface pe-7 ps-7 text-body text-fg-1 outline-none hover:bg-subtle focus:border-ring focus:ring-2 focus:ring-ring/20"
      >
        <option value="">Choose a project…</option>
        {projects
          .filter((project) => !project.hidden)
          .map((project) => (
            <option key={project.id} value={project.id}>
              {project.name}
            </option>
          ))}
      </select>
      <ChevronDown className="pointer-events-none absolute end-2 h-3.5 w-3.5 text-fg-3" />
    </label>
  );
}
