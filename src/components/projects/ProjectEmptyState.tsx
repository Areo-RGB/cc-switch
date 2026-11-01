import { Notice } from "@/components/ui/notice";
import { Button } from "@/components/ui/button";

export function ProjectEmptyState({ onChoose }: { onChoose: () => void }) {
  return (
    <div className="flex flex-1 items-start px-6 py-4">
      <Notice
        title="Pick a project to see its MCP servers and Skills"
        actions={
          <Button variant="neutral" size="compact" onClick={onChoose}>
            Choose project
          </Button>
        }
      />
    </div>
  );
}
