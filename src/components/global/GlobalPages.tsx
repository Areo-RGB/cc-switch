import type { ReactNode } from "react";
import { BookOpen, History, Settings2 } from "lucide-react";
import { AppPageHeader } from "@/components/shell/AppPageHeader";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function PromptsPage() {
  return (
    <SimplePage
      icon={<BookOpen className="h-5 w-5" strokeWidth={1.5} />}
      title="Prompts"
      description="Global prompt library shared across your projects."
      rows={["Release checklist", "Code review", "Explain this code"]}
      action="New prompt"
    />
  );
}
export function SessionsPage() {
  return (
    <SimplePage
      icon={<History className="h-5 w-5" strokeWidth={1.5} />}
      title="Sessions"
      description="Global session history across your agent clients."
      rows={[
        "Project setup · cc-switch",
        "Review GitHub integration",
        "Video pipeline notes",
      ]}
      action="Refresh sessions"
    />
  );
}
export function SettingsPage() {
  return (
    <SimplePage
      icon={<Settings2 className="h-5 w-5" strokeWidth={1.5} />}
      title="Settings"
      description="Application preferences stay global; project behavior lives on each project page."
      rows={["General", "Data & backups", "About"]}
      action="Save changes"
    />
  );
}

function SimplePage({
  icon,
  title,
  description,
  rows,
  action,
}: {
  icon: ReactNode;
  title: string;
  description: string;
  rows: string[];
  action: string;
}) {
  return (
    <>
      <AppPageHeader
        icon={icon}
        title={title}
        actions={
          <Button variant="solid" size="regular">
            {action}
          </Button>
        }
      />
      <main
        id="main-content"
        className="min-h-0 flex-1 overflow-y-auto px-6 pb-12 pt-5"
      >
        <div className="mx-auto max-w-[860px] space-y-4">
          <p className="m-0 text-body text-fg-2">{description}</p>
          <Card>
            <CardHeader className="p-5 pb-3">
              <CardTitle className="text-body font-semibold">
                {title} library
              </CardTitle>
            </CardHeader>
            <CardContent className="p-0">
              {rows.map((row) => (
                <div
                  key={row}
                  className="flex items-center justify-between border-t border-border px-5 py-3 text-body first:border-t-0"
                >
                  <span>{row}</span>
                  <Button variant="quiet" size="compact">
                    Open
                  </Button>
                </div>
              ))}
            </CardContent>
          </Card>
        </div>
      </main>
    </>
  );
}
