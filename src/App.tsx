import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Sidebar } from "@/components/shell/Sidebar";
import { projectsApi, type Project } from "@/lib/api/projects";
import {
  readStoredView,
  storeView,
  type GlobalPage,
  type View,
} from "@/lib/navigation";
import { ProjectPage } from "@/components/projects/ProjectPage";
import { ProjectPickerPage } from "@/components/projects/ProjectPickerPage";
import { ProjectMcpPanel } from "@/components/mcp/ProjectMcpPanel";
import { ProjectSkillsPanel } from "@/components/skills/ProjectSkillsPanel";
import { VideoPage } from "@/components/video/VideoPage";
import {
  PromptsPage,
  SessionsPage,
  SettingsPage,
} from "@/components/global/GlobalPages";
import { toast } from "@/lib/toast";

function App() {
  const [view, setView] = useState<View>(readStoredView);
  const [activeProjectId, setActiveProjectId] = useState<string | null>(null);
  const projectsQuery = useQuery({
    queryKey: ["projects"],
    queryFn: () => projectsApi.list(true),
  });
  const [projects, setProjects] = useState<Project[]>([]);

  useEffect(() => {
    if (projectsQuery.data) setProjects(projectsQuery.data);
  }, [projectsQuery.data]);
  useEffect(() => {
    storeView(view);
  }, [view]);

  const activeProject = useMemo(
    () => projects.find((project) => project.id === activeProjectId) ?? null,
    [activeProjectId, projects],
  );
  const selectProject = (project: Project) => {
    setActiveProjectId(project.id);
    try {
      localStorage.setItem("cc-switch-active-project", project.id);
    } catch {
      /* optional */
    }
    setView("project");
  };
  const changeProject = (projectId: string | null) => {
    if (!projectId) {
      setActiveProjectId(null);
      setView("projects");
      return;
    }
    const project = projects.find((item) => item.id === projectId);
    if (project) selectProject(project);
  };
  const addFolder = async () => {
    const path = window.prompt("Folder path");
    if (!path?.trim()) return;
    try {
      const project = await projectsApi.addFolder(path.trim());
      setProjects((current) => [...current, project]);
      toast.success(`Added ${project.name}`);
    } catch (error) {
      toast.error(String(error));
    }
  };
  const toggleHidden = async (project: Project) => {
    const hidden = !project.hidden;
    setProjects((current) =>
      current.map((item) =>
        item.id === project.id ? { ...item, hidden } : item,
      ),
    );
    try {
      await projectsApi.setHidden(project.id, hidden);
      toast.success(
        hidden ? `${project.name} hidden` : `${project.name} shown`,
      );
    } catch (error) {
      toast.error(String(error));
    }
  };
  const selectPage = (page: GlobalPage | "projects" | "settings") =>
    setView(page);
  const pickProject = () => setView("projects");

  return (
    <div className="flex h-screen overflow-hidden bg-app text-fg-1 selection:bg-action/25">
      <Sidebar
        projects={projects}
        activeProjectId={activeProjectId}
        view={view}
        settingsSection="general"
        onSelectProject={selectProject}
        onAddFolder={() => void addFolder()}
        onToggleHidden={(project) => void toggleHidden(project)}
        onSelectPage={selectPage}
      />
      <main
        id="content-area"
        className="relative isolate flex min-w-0 flex-1 flex-col overflow-hidden bg-app"
      >
        {view === "projects" && (
          <ProjectPickerPage
            projects={projects}
            onSelect={selectProject}
            onAddFolder={() => void addFolder()}
          />
        )}
        {view === "project" &&
          (activeProject ? (
            <ProjectPage
              projects={projects}
              project={activeProject}
              onProjectChange={changeProject}
            />
          ) : (
            <ProjectPickerPage
              projects={projects}
              onSelect={selectProject}
              onAddFolder={() => void addFolder()}
            />
          ))}
        {view === "mcp" && (
          <ProjectMcpPanel
            projects={projects}
            projectId={activeProjectId}
            onProjectChange={changeProject}
            onPickProject={pickProject}
          />
        )}
        {view === "skills" && (
          <ProjectSkillsPanel
            projects={projects}
            projectId={activeProjectId}
            onProjectChange={changeProject}
            onPickProject={pickProject}
          />
        )}
        {view === "video" && <VideoPage />}
        {view === "prompts" && <PromptsPage />}
        {view === "sessions" && <SessionsPage />}
        {view === "settings" && <SettingsPage />}
      </main>
    </div>
  );
}

export default App;
