import { useMemo, useState } from "react";
import { Filter, Plus } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { useI18n } from "@/hooks/useI18n";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useCatalog } from "@/hooks/useCatalog";
import * as api from "@/lib/api";
import type { Task } from "@/lib/types";

/** First-class work items, deliberately kept separate from time-entry editing. */
export function Tasks() {
  const { t } = useI18n();
  const catalog = useCatalog();
  const [title, setTitle] = useState("");
  const [filter, setFilter] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [projectFilter, setProjectFilter] = useState("");
  const [newProjectId, setNewProjectId] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const query = filter.trim().toLowerCase();
  const tasks = useMemo(
    () =>
      catalog.tasks.filter(
        (task) =>
          (statusFilter === "" || task.status === statusFilter) &&
          (projectFilter === "" || task.projectId === projectFilter) &&
          (!query ||
            task.title.toLowerCase().includes(query) ||
            task.keywords.some((keyword) => keyword.toLowerCase().includes(query)) ||
            task.labels.some((label) => label.name.toLowerCase().includes(query))),
      ),
    [catalog.tasks, projectFilter, query, statusFilter],
  );

  async function addTask() {
    const trimmed = title.trim();
    const selectedProject = newProjectId || projectFilter || catalog.projects[0]?.id;
    if (!trimmed || !selectedProject) return;
    setCreating(true);
    try {
      await api.createTask({ projectId: selectedProject, title: trimmed });
      setTitle("");
      setNewProjectId("");
      setDialogOpen(false);
      await catalog.reload();
    } finally {
      setCreating(false);
    }
  }

  async function advance(task: Task) {
    const status =
      task.status === "todo" ? "in_progress" : task.status === "in_progress" ? "done" : "todo";
    await api.updateTask(task.id, { status });
    await catalog.reload();
  }

  return (
    <main className="flex min-h-0 flex-1 flex-col overflow-y-auto bg-canvas p-5 text-fg">
      <header className="mb-5 flex flex-wrap items-end justify-between gap-3">
        <div>
          <p className="text-[11px] font-semibold tracking-[0.18em] text-fg-faint uppercase">
            Work management
          </p>
          <h1 className="mt-1 font-semibold text-[22px] text-fg-strong">{t("tasks.tasks")}</h1>
          <p className="mt-1 text-[13px] text-fg-muted">
            Client → project → task, with labels and AI instructions ready for integrations.
          </p>
        </div>
        <Button className="gap-1.5" onClick={() => setDialogOpen(true)}>
          <Plus className="size-3.5" /> New task
        </Button>
      </header>

      <div className="mb-4 flex flex-wrap items-center gap-2">
        <Input
          className="min-w-56 max-w-sm flex-1"
          aria-label="Search tasks"
          placeholder={t("tasks.searchTasks")}
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
        />
        <select
          aria-label="Filter tasks by status"
          className="h-8 rounded-lg border border-line bg-panel px-2 text-xs text-fg-strong outline-none focus:border-accent"
          value={statusFilter}
          onChange={(event) => setStatusFilter(event.target.value)}
        >
          <option value="">{t("tasks.allStatuses")}</option>
          <option value="todo">{t("tasks.todo")}</option>
          <option value="in_progress">{t("tasks.inProgress")}</option>
          <option value="blocked">{t("tasks.blocked")}</option>
          <option value="done">{t("tasks.done")}</option>
        </select>
        <select
          aria-label="Filter tasks by project"
          className="h-8 rounded-lg border border-line bg-panel px-2 text-xs text-fg-strong outline-none focus:border-accent"
          value={projectFilter}
          onChange={(event) => setProjectFilter(event.target.value)}
        >
          <option value="">{t("tasks.allProjects")}</option>
          {catalog.projects.map((project) => (
            <option key={project.id} value={project.id}>
              {project.name}
            </option>
          ))}
        </select>
        <Button variant="outline" size="sm" className="gap-1.5">
          <Filter className="size-3.5" /> Filters
        </Button>
      </div>

      <div className="grid gap-2">
        {tasks.map((task) => {
          const project = catalog.projectById.get(task.projectId);
          return (
            <Card key={task.id} size="sm">
              <CardContent className="flex items-center gap-3">
                <Button variant="outline" size="sm" onClick={() => void advance(task)}>
                  {task.status.replace("_", " ")}
                </Button>
                <div className="min-w-0 flex-1">
                  <p className="truncate font-medium text-fg-strong">{task.title}</p>
                  <p className="text-[12px] text-fg-muted">
                    {project?.name ?? "Unknown project"}
                    {task.keywords.length ? ` · ${task.keywords.join(", ")}` : ""}
                  </p>
                </div>
                <div className="flex gap-1">
                  {task.labels.map((label) => (
                    <span
                      key={label.id}
                      className="rounded-full px-2 py-0.5 text-[11px]"
                      style={{ backgroundColor: `${label.color}22`, color: label.color }}
                    >
                      {label.name}
                    </span>
                  ))}
                </div>
              </CardContent>
            </Card>
          );
        })}
        {tasks.length === 0 && (
          <p className="rounded-lg border border-dashed border-line p-6 text-center text-sm text-fg-muted">
            No tasks match the current filters.
          </p>
        )}
      </div>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("tasks.newTask")}</DialogTitle>
            <DialogDescription>Create a persistent task in an existing project.</DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <Input
              aria-label="Task title"
              placeholder={t("tasks.whatNeedsDone")}
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") void addTask();
              }}
            />
            <select
              aria-label="Project for new task"
              className="h-8 w-full rounded-lg border border-line bg-panel px-2 text-xs text-fg-strong outline-none focus:border-accent"
              value={newProjectId || projectFilter || catalog.projects[0]?.id || ""}
              onChange={(event) => setNewProjectId(event.target.value)}
            >
              {catalog.projects.map((project) => (
                <option key={project.id} value={project.id}>
                  {project.name}
                </option>
              ))}
            </select>
          </div>
          <DialogFooter showCloseButton>
            <Button
              disabled={!title.trim() || catalog.projects.length === 0 || creating}
              onClick={() => void addTask()}
            >
              {creating ? "Creating…" : "Create task"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </main>
  );
}
