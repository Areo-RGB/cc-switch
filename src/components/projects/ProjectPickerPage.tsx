import {
  FolderGit2,
  FolderPlus,
  GitBranch,
  MoreHorizontal,
} from "lucide-react";
import type { Project } from "@/lib/api/projects";
import { AppPageHeader } from "@/components/shell/AppPageHeader";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function ProjectPickerPage({
  projects,
  onSelect,
  onAddFolder,
}: {
  projects: Project[];
  onSelect: (project: Project) => void;
  onAddFolder: () => void;
}) {
  return (
    <>
      <AppPageHeader
        icon={<FolderGit2 className="h-5 w-5" strokeWidth={1.5} />}
        title="Projects"
        actions={
          <Button variant="solid" size="regular" onClick={onAddFolder}>
            <FolderPlus className="h-4 w-4" />
            Add folder
          </Button>
        }
      />
      <main
        id="main-content"
        className="min-h-0 flex-1 overflow-y-auto px-6 pb-12 pt-5"
      >
        <div className="mx-auto max-w-[1080px] space-y-5">
          <div>
            <p className="m-0 text-body text-fg-2">
              Choose a workspace to open its overview, Git status, scripts, and
              agent setup.
            </p>
          </div>
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {projects
              .filter((project) => !project.hidden)
              .map((project) => (
                <Card
                  key={project.id}
                  className="cursor-pointer transition-colors hover:border-border-strong hover:bg-subtle"
                  onClick={() => onSelect(project)}
                >
                  <CardHeader className="gap-3 p-5 pb-3">
                    <div className="flex items-start justify-between gap-3">
                      <div className="flex min-w-0 items-center gap-2.5">
                        <span className="flex h-8 w-8 items-center justify-center rounded-control bg-subtle text-fg-2">
                          <FolderGit2 className="h-4 w-4" />
                        </span>
                        <div className="min-w-0">
                          <CardTitle className="truncate text-body font-semibold">
                            {project.name}
                          </CardTitle>
                          <p className="m-0 truncate text-caption text-fg-3">
                            {project.path}
                          </p>
                        </div>
                      </div>
                      <MoreHorizontal className="h-4 w-4 shrink-0 text-fg-3" />
                    </div>
                  </CardHeader>
                  <CardContent className="space-y-3 p-5 pt-1">
                    <div className="flex flex-wrap gap-1.5">
                      {project.frameworks.map((framework) => (
                        <span
                          key={framework}
                          className="rounded-full bg-subtle px-2 py-0.5 text-caption text-fg-2"
                        >
                          {framework}
                        </span>
                      ))}
                    </div>
                    <div className="flex items-center gap-2 text-caption text-fg-2">
                      <GitBranch className="h-3.5 w-3.5" />
                      {project.branch}
                      <span className="text-fg-3">·</span>
                      {project.changedFiles} changed
                    </div>
                  </CardContent>
                </Card>
              ))}
          </div>
        </div>
      </main>
    </>
  );
}
