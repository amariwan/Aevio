import { useEffect, useState } from "react";
import { Flag, FolderKanban, Plus, Target } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/hooks/useI18n";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PageHeader } from "@/components/layout/Page";
import { useCatalog } from "@/hooks/useCatalog";
import { createInitiative, listInitiatives } from "@/lib/api";
import type { WorkspaceInitiative } from "@/lib/types";

export function Initiatives() {
  const { t } = useI18n();
  const catalog = useCatalog();
  const [initiatives, setInitiatives] = useState<WorkspaceInitiative[]>([]);
  useEffect(() => {
    void listInitiatives().then(setInitiatives);
  }, []);
  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-canvas text-fg">
      <PageHeader title="Initiatives">
        <Button
          size="sm"
          className="gap-1.5"
          onClick={() => {
            void createInitiative({
              name: `Initiative ${initiatives.length + 1}`,
              description: "Outcome across projects",
              projectIds: catalog.projects.map((project) => project.id),
            }).then((initiative) => setInitiatives((current) => [initiative, ...current]));
          }}
        >
          <Plus className="size-3.5" /> New initiative
        </Button>
      </PageHeader>
      <main className="min-h-0 flex-1 overflow-y-auto p-5">
        <div className="mx-auto max-w-5xl space-y-5">
          <div>
            <p className="text-[11px] font-semibold tracking-[0.18em] text-fg-faint uppercase">
              Direction
            </p>
            <h1 className="mt-1 text-2xl font-semibold text-fg-strong">
              Connect projects to outcomes.
            </h1>
            <p className="mt-1 text-sm text-fg-muted">
              Initiatives are the persisted cross-project layer above the existing Aevio project
              model.
            </p>
          </div>
          <div className="grid gap-3">
            {initiatives.map((initiative) => (
              <Card key={initiative.id} className="border-line bg-panel">
                <CardHeader className="flex-row items-start justify-between">
                  <div>
                    <CardTitle className="text-sm text-fg-strong">{initiative.name}</CardTitle>
                    <p className="mt-1 text-xs text-fg-muted">{initiative.description}</p>
                  </div>
                </CardHeader>
                <CardContent className="text-xs text-fg-muted">
                  {initiative.projectIds.length} linked projects · {initiative.status}
                </CardContent>
              </Card>
            ))}
            {catalog.projects.map((project) => {
              const tasks = catalog.tasks.filter((task) => task.projectId === project.id);
              const done = tasks.filter((task) => task.status === "done").length;
              const progress = tasks.length ? Math.round((done / tasks.length) * 100) : 0;
              return (
                <Card key={project.id} className="border-line bg-panel">
                  <CardHeader className="flex-row items-start justify-between">
                    <div className="flex gap-3">
                      <span className="grid size-9 place-items-center rounded-xl bg-accent-soft text-accent">
                        <Flag className="size-4" />
                      </span>
                      <div>
                        <CardTitle className="text-sm text-fg-strong">{project.name}</CardTitle>
                        <p className="mt-1 text-xs text-fg-muted">
                          Project initiative · {tasks.length} linked tasks
                        </p>
                      </div>
                    </div>
                    <Badge variant={project.status === "active" ? "default" : "outline"}>
                      {project.status}
                    </Badge>
                  </CardHeader>
                  <CardContent>
                    <div className="mb-2 flex items-center justify-between text-xs">
                      <span className="text-fg-muted">{t("initiatives.outcomeProgress")}</span>
                      <span className="font-mono text-fg-strong">{progress}%</span>
                    </div>
                    <div className="h-1.5 overflow-hidden rounded-full bg-surface-strong">
                      <div
                        className="h-full rounded-full bg-accent"
                        style={{ width: `${progress}%` }}
                      />
                    </div>
                    <div className="mt-4 flex flex-wrap gap-3 text-xs text-fg-muted">
                      <span className="flex items-center gap-1">
                        <FolderKanban className="size-3.5" /> {project.name}
                      </span>
                      <span className="flex items-center gap-1">
                        <Target className="size-3.5" /> {done} completed
                      </span>
                    </div>
                  </CardContent>
                </Card>
              );
            })}
            {catalog.projects.length === 0 && <Empty />}
          </div>
        </div>
      </main>
    </div>
  );
}

function Empty() {
  return (
    <div className="rounded-xl border border-dashed border-line p-8 text-center text-sm text-fg-muted">
      Create a project before adding an initiative.
    </div>
  );
}
