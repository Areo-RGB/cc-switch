export type GlobalPage =
  | "mcp"
  | "skills"
  | "prompts"
  | "sessions"
  | "video"
  | "usage"
  | "auth"
  | "apps"
  | "skillsDiscovery";
export type View =
  | "projects"
  | "project"
  | GlobalPage
  | "settings"
  | "providers"
  | "workspace"
  | "openclawConfig"
  | "hermesMemory";
export type SettingsSection =
  | "general"
  | "data"
  | "about"
  | "appConfig"
  | "routing"
  | "network";

export const SETTINGS_SECTIONS: SettingsSection[] = [
  "general",
  "appConfig",
  "routing",
  "network",
  "data",
  "about",
];
export const VIEW_STORAGE_KEY = "cc-switch-last-view";
export const ACTIVE_PROJECT_STORAGE_KEY = "cc-switch-active-project";

export function parseView(value: string | null | undefined): View | null {
  if (value === "projects" || value === "project" || value === "settings")
    return value;
  if (
    value === "mcp" ||
    value === "skills" ||
    value === "prompts" ||
    value === "sessions" ||
    value === "video"
  )
    return value;
  return null;
}

export function readStoredView(): View {
  try {
    return parseView(localStorage.getItem(VIEW_STORAGE_KEY)) ?? "projects";
  } catch {
    return "projects";
  }
}

export function storeView(view: View) {
  try {
    localStorage.setItem(VIEW_STORAGE_KEY, view);
  } catch {
    // Browser storage is optional; the current view still works in memory.
  }
}
