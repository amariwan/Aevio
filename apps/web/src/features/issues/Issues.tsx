import { useEffect, useMemo, useState } from "react";
import {
  ChevronRight,
  CircleDot,
  Filter,
  LayoutGrid,
  List,
  MessageSquare,
  Plus,
  Send,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { PageHeader } from "@/components/layout/Page";
import { useCatalog } from "@/hooks/useCatalog";
import { useI18n } from "@/hooks/useI18n";
import * as api from "@/lib/api";
import type { Task, WorkspaceComment } from "@/lib/types";

type IssueStatus = Task["status"];

type ColumnDef = { status: IssueStatus; label: string };
function columns(t: (k: any) => string): ColumnDef[] {
  return [
    { status: "todo", label: t("issues.backlog") },
    { status: "in_progress", label: t("issues.inProgress") },
    { status: "done", label: "Done" },
  ];
}

const STATUS_TONES: Record<IssueStatus, string> = {
  todo: "bg-fg-faint",
  in_progress: "bg-accent",
  done: "bg-success",
};

export function Issues() {
  const catalog = useCatalog();
  const { t } = useI18n();
  const [query, setQuery] = useState("");
  const [projectId, setProjectId] = useState("");
  const [statusFilter, setStatusFilter] = useState<"" | IssueStatus>("");
  const [labelFilter, setLabelFilter] = useState("");
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [view, setView] = useState<"board" | "table">("board");
  const [title, setTitle] = useState("");
  const [creating, setCreating] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [newProjectId, setNewProjectId] = useState("");
  const [selected, setSelected] = useState<Task | null>(null);

  const issues = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return catalog.tasks.filter((task) => {
      const matchesProject = projectId === "" || task.projectId === projectId;
      const matchesStatus = statusFilter === "" || task.status === statusFilter;
      const matchesLabel =
        labelFilter === "" || task.labels.some((label) => label.id === labelFilter);
      const matchesQuery =
        normalized === "" ||
        task.title.toLowerCase().includes(normalized) ||
        task.keywords.some((keyword) => keyword.toLowerCase().includes(normalized)) ||
        task.labels.some((label) => label.name.toLowerCase().includes(normalized));
      return matchesProject && matchesStatus && matchesLabel && matchesQuery;
    });
  }, [catalog.tasks, labelFilter, projectId, query, statusFilter]);

  async function createIssue(): Promise<void> {
    const trimmed = title.trim();
    const selectedProject = newProjectId || projectId || catalog.projects[0]?.id;
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

  async function moveIssue(task: Task): Promise<void> {
    const nextStatus: IssueStatus =
      task.status === "todo" ? "in_progress" : task.status === "in_progress" ? "done" : "todo";
    await api.updateTask(task.id, { status: nextStatus });
    await catalog.reload();
    setSelected((current) => (current?.id === task.id ? { ...task, status: nextStatus } : current));
  }

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-canvas text-fg">
      <PageHeader title={t("issues.issues")}>
        <div className="flex items-center rounded-lg border border-line bg-surface p-0.5">
          <Button
            variant={view === "board" ? "secondary" : "ghost"}
            size="icon-xs"
            aria-label={t("issues.kanbanView")}
            onClick={() => setView("board")}
          >
            <LayoutGrid className="size-3.5" />
          </Button>
          <Button
            variant={view === "table" ? "secondary" : "ghost"}
            size="icon-xs"
            aria-label={t("issues.tableView")}
            onClick={() => setView("table")}
          >
            <List className="size-3.5" />
          </Button>
        </div>
        <Button
          variant={filtersOpen ? "secondary" : "outline"}
          size="sm"
          className="gap-1.5"
          onClick={() => setFiltersOpen((open) => !open)}
        >
          <Filter className="size-3.5" /> Filters
        </Button>
        <Button size="sm" className="gap-1.5" onClick={() => setDialogOpen(true)}>
          <Plus className="size-3.5" /> New issue
        </Button>
      </PageHeader>

      <main className="min-h-0 flex-1 overflow-y-auto p-5">
        <div className="mx-auto max-w-[1500px] space-y-4">
          <div className="flex flex-wrap items-center gap-2">
            <Input
              className="min-w-56 max-w-sm flex-1"
              aria-label={t("issues.search")}
              placeholder={t("issues.searchIssues")}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
            <select
              aria-label={t("issues.filterByProject")}
              className="h-8 rounded-lg border border-line bg-panel px-2 text-xs text-fg-strong outline-none focus:border-accent"
              value={projectId}
              onChange={(event) => setProjectId(event.target.value)}
            >
              <option value="">{t("issues.allProjects")}</option>
              {catalog.projects.map((project) => (
                <option key={project.id} value={project.id}>
                  {project.name}
                </option>
              ))}
            </select>
          </div>

          {filtersOpen && (
            <div className="flex flex-wrap items-end gap-3 rounded-xl border border-line bg-surface p-3">
              <FilterSelect
                label={t("issues.status")}
                value={statusFilter}
                onChange={(value) => setStatusFilter(value as "" | IssueStatus)}
                options={columns(t).map((column) => ({
                  value: column.status,
                  label: column.label,
                }))}
              />
              <FilterSelect
                label={t("issues.label")}
                value={labelFilter}
                onChange={setLabelFilter}
                options={catalog.labels.map((label) => ({ value: label.id, label: label.name }))}
              />
              {(statusFilter || labelFilter || projectId || query) && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="mb-0.5"
                  onClick={() => {
                    setQuery("");
                    setProjectId("");
                    setStatusFilter("");
                    setLabelFilter("");
                  }}
                >
                  Clear filters
                </Button>
              )}
            </div>
          )}

          {view === "board" ? (
            <div className="grid min-h-[420px] gap-3 lg:grid-cols-3">
              {columns(t).map((column) => {
                const columnIssues = issues.filter((issue) => issue.status === column.status);
                return (
                  <section
                    key={column.status}
                    className="rounded-xl border border-line bg-rail/60 p-2"
                  >
                    <header className="flex items-center gap-2 px-2 py-2">
                      <span className={`size-2 rounded-full ${STATUS_TONES[column.status]}`} />
                      <h2 className="text-xs font-semibold text-fg-strong">{column.label}</h2>
                      <span className="font-mono text-[10px] text-fg-faint">
                        {columnIssues.length}
                      </span>
                    </header>
                    <div className="space-y-2">
                      {columnIssues.map((issue) => (
                        <IssueCard
                          key={issue.id}
                          issue={issue}
                          projectName={catalog.projectById.get(issue.projectId)?.name}
                          onOpen={() => setSelected(issue)}
                          onAdvance={() => void moveIssue(issue)}
                        />
                      ))}
                      {columnIssues.length === 0 && (
                        <p className="rounded-lg border border-dashed border-line p-5 text-center text-xs text-fg-faint">
                          No issues
                        </p>
                      )}
                    </div>
                  </section>
                );
              })}
            </div>
          ) : (
            <IssueTable
              issues={issues}
              projectById={catalog.projectById}
              onOpen={setSelected}
              onAdvance={(issue) => void moveIssue(issue)}
            />
          )}
        </div>
      </main>

      {selected && (
        <IssueDetail
          issue={selected}
          projectName={catalog.projectById.get(selected.projectId)?.name}
          onClose={() => setSelected(null)}
          onAdvance={() => void moveIssue(selected)}
        />
      )}

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("issues.newIssue")}</DialogTitle>
            <DialogDescription>Create a persistent issue in an existing project.</DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <Input
              aria-label={t("issues.issueTitle")}
              placeholder={t("issues.describeWork")}
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") void createIssue();
              }}
            />
            <select
              aria-label={t("issues.projectForNew")}
              className="h-8 w-full rounded-lg border border-line bg-panel px-2 text-xs text-fg-strong outline-none focus:border-accent"
              value={newProjectId || projectId || catalog.projects[0]?.id || ""}
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
              onClick={() => void createIssue()}
            >
              {creating ? "Creating…" : t("issues.createIssue")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function FilterSelect({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: Array<{ value: string; label: string }>;
}) {
  return (
    <label className="grid gap-1 text-[10px] font-semibold tracking-wide text-fg-faint uppercase">
      {label}
      <select
        className="h-8 min-w-36 rounded-lg border border-line bg-panel px-2 text-xs font-normal text-fg-strong outline-none focus:border-accent"
        value={value}
        onChange={(event) => onChange(event.target.value)}
      >
        <option value="">All {label.toLowerCase()}s</option>
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function IssueTable({
  issues,
  projectById,
  onOpen,
  onAdvance,
}: {
  issues: Task[];
  projectById: Map<string, { name: string }>;
  onOpen: (issue: Task) => void;
  onAdvance: (issue: Task) => void;
}) {
  const { t } = useI18n();
  return (
    <div className="overflow-hidden rounded-xl border border-line bg-panel">
      <table className="w-full text-left text-xs">
        <thead className="border-b border-line bg-surface text-[10px] tracking-wide text-fg-faint uppercase">
          <tr>
            <th className="px-4 py-3 font-semibold">{t("issues.issue")}</th>
            <th className="px-4 py-3 font-semibold">{t("issues.project")}</th>
            <th className="px-4 py-3 font-semibold">{t("issues.status")}</th>
            <th className="px-4 py-3 font-semibold">{t("issues.priority")}</th>
            <th className="px-4 py-3 font-semibold">{t("issues.labels")}</th>
            <th className="px-4 py-3" />
          </tr>
        </thead>
        <tbody className="divide-y divide-line">
          {issues.map((issue) => (
            <tr key={issue.id} className="transition-colors hover:bg-surface">
              <td className="max-w-sm px-4 py-3">
                <button
                  type="button"
                  className="truncate font-medium text-fg-strong hover:text-accent"
                  onClick={() => onOpen(issue)}
                >
                  {issue.title}
                </button>
              </td>
              <td className="px-4 py-3 text-fg-muted">
                {projectById.get(issue.projectId)?.name ?? t("issues.unknownProject")}
              </td>
              <td className="px-4 py-3">
                <span className="inline-flex items-center gap-1.5 capitalize text-fg-muted">
                  <span
                    className={`size-1.5 rounded-full ${STATUS_TONES[issue.status as IssueStatus] ?? "bg-fg-faint"}`}
                  />
                  {issue.status.replace("_", " ")}
                </span>
              </td>
              <td className="px-4 py-3 capitalize text-fg-muted">{issue.priority}</td>
              <td className="px-4 py-3">
                <div className="flex gap-1">
                  {issue.labels.slice(0, 2).map((label) => (
                    <Badge key={label.id} variant="outline" className="h-5 px-1.5 text-[10px]">
                      {label.name}
                    </Badge>
                  ))}
                </div>
              </td>
              <td className="px-4 py-3 text-right">
                <Button
                  variant="ghost"
                  size="icon-xs"
                  onClick={() => onAdvance(issue)}
                  aria-label={`Advance ${issue.title}`}
                >
                  <ChevronRight className="size-3.5" />
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {issues.length === 0 && (
        <p className="p-8 text-center text-xs text-fg-faint">
          No issues match the current filters.
        </p>
      )}
    </div>
  );
}

function IssueCard({
  issue,
  projectName,
  onOpen,
  onAdvance,
}: {
  issue: Task;
  projectName?: string;
  onOpen: () => void;
  onAdvance: () => void;
}) {
  const { t } = useI18n();
  return (
    <Card className="border-line bg-panel transition-colors hover:border-line-strong">
      <CardContent className="p-3">
        <button type="button" onClick={onOpen} className="block w-full text-left">
          <p className="text-[13px] font-medium leading-snug text-fg-strong">{issue.title}</p>
          <p className="mt-1 truncate text-[11px] text-fg-muted">
            {projectName ?? t("issues.unknownProject")}
          </p>
        </button>
        <div className="mt-3 flex items-center gap-1.5">
          {issue.labels.slice(0, 3).map((label) => (
            <Badge
              key={label.id}
              variant="outline"
              className="h-5 px-1.5 text-[10px]"
              style={{ borderColor: label.color, color: label.color }}
            >
              {label.name}
            </Badge>
          ))}
          <Button
            variant="ghost"
            size="icon-xs"
            className="ml-auto"
            onClick={onAdvance}
            aria-label={`Advance ${issue.title}`}
          >
            <ChevronRight className="size-3.5" />
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function IssueDetail({
  issue,
  projectName,
  onClose,
  onAdvance,
}: {
  issue: Task;
  projectName?: string;
  onClose: () => void;
  onAdvance: () => void;
}) {
  const { t } = useI18n();
  const [comments, setComments] = useState<WorkspaceComment[]>([]);
  const [comment, setComment] = useState("");
  const [commentsLoading, setCommentsLoading] = useState(true);
  const [commentSaving, setCommentSaving] = useState(false);

  useEffect(() => {
    let active = true;
    setCommentsLoading(true);
    void api
      .listComments("task", issue.id)
      .then((next) => {
        if (active) setComments(next);
      })
      .finally(() => {
        if (active) setCommentsLoading(false);
      });
    return () => {
      active = false;
    };
  }, [issue.id]);

  async function submitComment(): Promise<void> {
    const body = comment.trim();
    if (!body || commentSaving) return;
    setCommentSaving(true);
    try {
      const created = await api.createComment({
        targetType: "task",
        targetId: issue.id,
        body,
        author: "You",
      });
      setComments((current) => [...current, created]);
      setComment("");
    } finally {
      setCommentSaving(false);
    }
  }

  return (
    <aside className="fixed inset-y-0 right-0 z-40 flex w-full max-w-md flex-col border-l border-line bg-panel shadow-2xl">
      <div className="flex items-start justify-between border-b border-line p-5">
        <div className="min-w-0">
          <p className="font-mono text-[11px] text-fg-faint">{t("issues.issue")}</p>
          <h2 className="mt-1 text-lg font-semibold text-fg-strong">{issue.title}</h2>
          <p className="mt-1 text-xs text-fg-muted">{projectName ?? t("issues.unknownProject")}</p>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={onClose}
          aria-label={t("issues.closeIssue")}
        >
          ×
        </Button>
      </div>
      <div className="flex-1 space-y-5 overflow-y-auto p-5">
        <div className="grid grid-cols-2 gap-2 text-xs">
          <Meta label={t("issues.status")} value={issue.status.replace("_", " ")} />
          <Meta
            label={t("issues.keywords")}
            value={issue.keywords.length ? issue.keywords.join(", ") : "None"}
          />
        </div>
        <section className="space-y-3">
          <div className="flex items-center gap-2">
            <MessageSquare className="size-4 text-fg-muted" />
            <h3 className="text-xs font-semibold text-fg-strong">{t("issues.comments")}</h3>
            <span className="font-mono text-[10px] text-fg-faint">{comments.length}</span>
          </div>
          {commentsLoading ? (
            <p className="text-xs text-fg-faint">Loading comments…</p>
          ) : comments.length === 0 ? (
            <p className="rounded-lg border border-dashed border-line p-3 text-xs text-fg-faint">
              Start the discussion on this issue.
            </p>
          ) : (
            <div className="space-y-2">
              {comments.map((item) => (
                <div key={item.id} className="rounded-lg border border-line bg-surface p-3">
                  <div className="flex items-center justify-between gap-2 text-[10px] text-fg-faint">
                    <span className="font-semibold text-fg-muted">{item.author}</span>
                    <span>{new Date(item.createdAt).toLocaleString()}</span>
                  </div>
                  <p className="mt-1 whitespace-pre-wrap text-xs leading-relaxed text-fg-muted">
                    {item.body}
                  </p>
                </div>
              ))}
            </div>
          )}
          <div className="space-y-2">
            <Textarea
              aria-label={t("issues.comment")}
              placeholder={t("issues.writeComment")}
              value={comment}
              onChange={(event) => setComment(event.target.value)}
              onKeyDown={(event) => {
                if ((event.metaKey || event.ctrlKey) && event.key === "Enter") void submitComment();
              }}
              rows={3}
            />
            <div className="flex items-center justify-between gap-2">
              <p className="text-[10px] text-fg-faint">⌘/Ctrl + Enter to send</p>
              <Button
                size="sm"
                className="gap-1.5"
                disabled={!comment.trim() || commentSaving}
                onClick={() => void submitComment()}
              >
                <Send className="size-3.5" /> {commentSaving ? "Sending…" : t("issues.comment")}
              </Button>
            </div>
          </div>
        </section>
        <div>
          <p className="mb-2 text-xs font-semibold text-fg-strong">{t("issues.labels")}</p>
          <div className="flex flex-wrap gap-1.5">
            {issue.labels.map((label) => (
              <Badge
                key={label.id}
                variant="outline"
                style={{ borderColor: label.color, color: label.color }}
              >
                {label.name}
              </Badge>
            ))}
          </div>
        </div>
      </div>
      <div className="border-t border-line p-4">
        <Button className="w-full gap-2" onClick={onAdvance}>
          <CircleDot className="size-4" /> Advance status
        </Button>
      </div>
    </aside>
  );
}

function Meta({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-line bg-surface p-2">
      <p className="text-[10px] text-fg-faint uppercase">{label}</p>
      <p className="mt-1 font-medium capitalize text-fg-strong">{value}</p>
    </div>
  );
}
