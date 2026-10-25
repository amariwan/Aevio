import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Button } from "@/components/ui/button";
import AIContextMeter, { type ContextSegment } from "@/components/kokonutui/ai-context-meter";
import AI_Prompt, { type PromptCommand } from "@/components/kokonutui/ai-prompt";
import { useI18n } from "@/hooks/useI18n";
import { useSettings } from "@/hooks/useSettings";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import * as api from "@/lib/api";
import { formatDuration } from "@/lib/format";
import { PALETTE } from "@/lib/palette";
import type { AutomationSnapshot, DailyWorkflowSnapshot, DeliveryChannel } from "@/lib/types";

interface Message {
  role: "user" | "agent";
  text: string;
  source?: "local-read-only" | "local-ai" | "external-ai";
}
type Tab = "chat" | "routines" | "reports" | "dashboards";

const ESTIMATED_CONTEXT_LIMIT = 128_000;

function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4);
}

export function Agent({ navigate }: { navigate?: (route: import("@/lib/types").Route) => void }) {
  const { t } = useI18n();
  const { settings } = useSettings();
  const [tab, setTab] = useState<Tab>("chat");
  const [snapshot, setSnapshot] = useState<AutomationSnapshot | null>(null);
  const [dailyWorkflow, setDailyWorkflow] = useState<DailyWorkflowSnapshot | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [agentContext, setAgentContext] = useState<api.WorkAnswer["context"] | null>(null);
  const [lastSources, setLastSources] = useState<api.WorkSourceRef[]>([]);
  const [suggestions, setSuggestions] = useState<any[]>([]);
  const [projects, setProjects] = useState<any[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [prompt, setPrompt] = useState("");
  const [cadence, setCadence] = useState("daily");
  const [delivery, setDelivery] = useState<DeliveryChannel[]>(["in-app"]);
  const [dashboardName, setDashboardName] = useState("");
  const contextSegments = useMemo<ContextSegment[]>(() => {
    const latestAnswerIndex = messages.reduce(
      (index, item, currentIndex) => (item.role === "agent" ? currentIndex : index),
      -1,
    );
    const latestAnswer = latestAnswerIndex >= 0 ? messages[latestAnswerIndex].text : "";
    const conversation = messages
      .filter((_, index) => index !== latestAnswerIndex)
      .map((item) => `${item.role}: ${item.text}`)
      .join("\n");
    const sources = lastSources.map((source) => `${source.kind}: ${source.label}`).join("\n");

    return [
      { id: "conversation", label: t("agent.conversation"), tokens: estimateTokens(conversation) },
      { id: "sources", label: t("agent.retrievedContext"), tokens: estimateTokens(sources) },
      { id: "answer", label: t("agent.latestAnswer"), tokens: estimateTokens(latestAnswer) },
    ].filter((segment) => segment.tokens > 0);
  }, [lastSources, messages]);
  const load = async (): Promise<void> => {
    try {
      const [nextSnapshot, nextDailyWorkflow] = await Promise.all([
        api.automationSnapshot(),
        api.dailyWorkflowSnapshot(),
      ]);
      setSnapshot(nextSnapshot);
      setDailyWorkflow(nextDailyWorkflow);
    } catch (cause) {
      setError(api.describeError(cause));
    }
  };
  const runDailyWorkflow = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      const result = await api.runDailyWorkflow();
      setDailyWorkflow(result.snapshot);
      await load();
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  useEffect(() => {
    void load();
    void (async () => {
      try {
        const s = await api.workProjectSuggestions(Date.now() - 30 * 86400000);
        setSuggestions(s as any[]);
        const p = await api.listProjects();
        setProjects(p as any[]);
      } catch {
        /* ignore */
      }
    })();
  }, []);
  const send = async (
    value: string,
    _model: string,
    { command, files }: { command: PromptCommand | null; files: File[] },
  ): Promise<void> => {
    const text = value.trim().slice(0, 2000);
    if (!text || busy) return;
    const question = command ? `${command.label}: ${text}` : text;
    setError(null);
    setMessages((current) => [...current, { role: "user", text: question }]);
    if (files.length > 0) {
      setError(t("agent.attachmentsUnsupported"));
    }
    setBusy(true);
    try {
      const reply = await api.workAsk(question);
      setAgentContext(reply.context);
      setLastSources(reply.sources);
      setMessages((current) => [
        ...current,
        {
          role: "agent",
          text: reply.answer,
          source:
            reply.mode === "external-ai"
              ? "external-ai"
              : reply.mode === "local-ai"
                ? "local-ai"
                : "local-read-only",
        },
      ]);
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  const createRoutine = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!name.trim() || !prompt.trim()) return;
    setBusy(true);
    try {
      await api.createRoutine({ name, prompt, cadence, delivery });
      setName("");
      setPrompt("");
      await load();
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  const createReport = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!name.trim()) return;
    setBusy(true);
    try {
      await api.createReport({ name, filter: {}, cadence, delivery });
      setName("");
      await load();
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  const toggleDelivery = (channel: DeliveryChannel): void => {
    setDelivery((current) =>
      current.includes(channel)
        ? current.filter((item) => item !== channel)
        : [...current, channel],
    );
  };
  const createDashboard = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!dashboardName.trim()) return;
    setBusy(true);
    try {
      await api.createDashboard({ name: dashboardName, filter: {} });
      setDashboardName("");
      await load();
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  const runRoutine = async (id: string): Promise<void> => {
    setBusy(true);
    try {
      await api.runRoutine(id);
      await load();
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  const runReport = async (id: string): Promise<void> => {
    setBusy(true);
    try {
      await api.runReport(id);
      await load();
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  const formatDate = (value: number): string =>
    new Date(value).toLocaleString("de-DE", { dateStyle: "medium", timeStyle: "short" });
  return (
    <main className="min-h-0 flex-1 overflow-auto bg-canvas px-6 py-6">
      <div className="mx-auto flex max-w-5xl flex-col gap-5">
        <header>
          <p className="text-xs font-semibold tracking-[0.18em] text-accent uppercase">
            {t("agent.title")}
          </p>
          <h1 className="mt-1 text-2xl font-semibold text-fg-strong">
            {t("agent.routinesAndReports")}
          </h1>
          <p className="mt-1 text-sm text-fg-muted">{t("agent.routinesDescription")}</p>
        </header>
        {(!settings.aiProvider || !settings.aiBaseUrl || !settings.aiModel) && (
          <div
            className="rounded-lg border border-dashed border-line bg-surface px-4 py-3 text-sm text-fg-muted"
            role="status"
          >
            {t("agent.aiConnectionMissing")}{" "}
            {navigate && (
              <button
                type="button"
                className="font-medium text-fg underline-offset-2 hover:underline"
                onClick={() => navigate({ name: "settings" })}
              >
                {t("agent.openSettings")}
              </button>
            )}
          </div>
        )}
        <nav className="flex flex-wrap gap-2" aria-label="Agent sections">
          {(["chat", "routines", "reports", "dashboards"] as Tab[]).map((item) => (
            <Button
              key={item}
              variant={tab === item ? "default" : "outline"}
              onClick={() => setTab(item)}
            >
              {item === "chat" ? t("agent.homeChat") : item[0].toUpperCase() + item.slice(1)}
            </Button>
          ))}
        </nav>
        {error && (
          <p className="rounded-lg bg-danger-soft px-3 py-2 text-sm text-danger">{error}</p>
        )}
        {dailyWorkflow && (
          <Card className="border-line bg-surface">
            <CardHeader className="flex flex-row items-start justify-between gap-4">
              <div>
                <CardTitle>{t("agent.dailyWorkflowTitle")}</CardTitle>
                <CardDescription>{t("agent.dailyWorkflowDescription")}</CardDescription>
              </div>
              <Button type="button" onClick={() => void runDailyWorkflow()} disabled={busy}>
                {busy ? t("agent.running") : t("agent.runDailyWorkflow")}
              </Button>
            </CardHeader>
            <CardContent className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
              <div>
                <p className="text-xs text-fg-muted">{t("agent.tracked")}</p>
                <p className="text-lg font-semibold text-fg">
                  {formatDuration(dailyWorkflow.trackedMs)}
                </p>
              </div>
              <div>
                <p className="text-xs text-fg-muted">{t("agent.approved")}</p>
                <p className="text-lg font-semibold text-fg">
                  {formatDuration(dailyWorkflow.approvedMs)}
                </p>
              </div>
              <div>
                <p className="text-xs text-fg-muted">{t("agent.autoApproved")}</p>
                <p className="text-lg font-semibold text-fg">{dailyWorkflow.autoApprovedEntries}</p>
              </div>
              <div>
                <p className="text-xs text-fg-muted">{t("agent.pendingReview")}</p>
                <p className="text-lg font-semibold text-fg">{dailyWorkflow.pendingEntries}</p>
              </div>
              <div>
                <p className="text-xs text-fg-muted">{t("agent.readyToInvoice")}</p>
                <p className="text-lg font-semibold text-fg">
                  {formatDuration(dailyWorkflow.readyToInvoiceMs)}
                </p>
              </div>
            </CardContent>
          </Card>
        )}
        {tab === "chat" && (
          <Card className="border-line bg-surface">
            <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
              <div>
                <CardTitle>{t("agent.chatTitle")}</CardTitle>
                <CardDescription>{t("agent.chatDescription")}</CardDescription>
              </div>
              <AIContextMeter
                className="sm:max-w-xs"
                limit={ESTIMATED_CONTEXT_LIMIT}
                model={`${agentContext?.aiEngine ?? "Aevio Agent"} · ${t("agent.estimated")}`}
                segments={contextSegments}
              />
            </CardHeader>
            <CardContent className="flex flex-col gap-4">
              {agentContext && (
                <div className="grid gap-2 sm:grid-cols-3" aria-label="Agent context">
                  <div className="rounded-lg border border-line bg-surface-strong px-3 py-2">
                    <p className="text-xs text-fg-muted">{t("agent.openReviews")}</p>
                    <p className="text-lg font-semibold text-fg">{agentContext.pendingEntries}</p>
                  </div>
                  <div className="rounded-lg border border-line bg-surface-strong px-3 py-2">
                    <p className="text-xs text-fg-muted">{t("agent.activeProjects")}</p>
                    <p className="text-lg font-semibold text-fg">{agentContext.activeProjects}</p>
                  </div>
                  <div className="rounded-lg border border-line bg-surface-strong px-3 py-2">
                    <p className="text-xs text-fg-muted">{t("agent.aiEngine")}</p>
                    <p className="text-sm font-semibold text-fg">{agentContext.aiEngine}</p>
                  </div>
                </div>
              )}
              <div className="min-h-64 space-y-3">
                {messages.length === 0 && (
                  <div className="rounded-lg border border-dashed border-line px-4 py-5 text-sm text-fg-muted">
                    {t("agent.emptyChat")}
                  </div>
                )}
                {messages.map((item, index) => (
                  <div
                    key={`${item.role}-${index}`}
                    className={`max-w-[85%] rounded-xl px-3 py-2.5 text-sm ${item.role === "user" ? "ml-auto bg-accent text-accent-foreground" : "bg-surface-strong text-fg"}`}
                  >
                    <>
                      {item.text}
                      {item.role === "agent" && item.source && (
                        <span className="mt-1 block text-[11px] text-fg-muted">
                          {item.source === "external-ai"
                            ? t("agent.externalAi")
                            : item.source === "local-ai"
                              ? t("agent.onDeviceAi")
                              : t("agent.localContext")}
                        </span>
                      )}
                    </>
                  </div>
                ))}
              </div>
              {suggestions.length > 0 && (
                <div className="space-y-2 border-t border-line pt-3">
                  <h3 className="text-sm font-semibold text-fg">{t("agent.projectSuggestions")}</h3>
                  {suggestions.map((s) => (
                    <div
                      key={s.key}
                      className="rounded-lg border border-line bg-surface p-3 text-sm"
                    >
                      <p className="font-medium text-fg">{s.name}</p>
                      <p className="text-xs text-fg-muted">{s.evidence?.join(", ")}</p>
                      <div className="mt-2 flex flex-wrap gap-2">
                        <button
                          type="button"
                          className="rounded-md border border-line px-2 py-1 text-xs"
                          onClick={async () => {
                            try {
                              await api.createProject({ name: s.name, color: PALETTE[0] });
                              await api.workDismissSuggestion(s.key);
                              setSuggestions((cur) => cur.filter((x) => x.key !== s.key));
                            } catch (e) {
                              setError(api.describeError(e));
                            }
                          }}
                        >
                          {t("agent.confirm")}
                        </button>
                        <select
                          className="rounded-md border border-line bg-surface px-2 py-1 text-xs"
                          onChange={async (e) => {
                            const pid = e.target.value;
                            if (!pid) return;
                            try {
                              await api.workMatchExisting(s.key, pid);
                              setSuggestions((cur) => cur.filter((x) => x.key !== s.key));
                            } catch (err) {
                              setError(api.describeError(err));
                            }
                          }}
                        >
                          <option value="">{t("agent.correct")}</option>
                          {projects.map((p) => (
                            <option key={p.id} value={p.id}>
                              {p.name}
                            </option>
                          ))}
                        </select>
                        <button
                          type="button"
                          className="rounded-md border border-line px-2 py-1 text-xs"
                          onClick={async () => {
                            try {
                              await api.workDismissSuggestion(s.key);
                              setSuggestions((cur) => cur.filter((x) => x.key !== s.key));
                            } catch (e) {
                              setError(api.describeError(e));
                            }
                          }}
                        >
                          {t("agent.ignore")}
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
              {lastSources.length > 0 && (
                <div className="flex flex-wrap gap-2 pt-2">
                  {lastSources.map((source) => (
                    <button
                      key={`${source.kind}-${source.id}`}
                      type="button"
                      className="rounded-md border border-line bg-surface px-2 py-1 text-xs text-fg-muted hover:text-fg"
                      onClick={() => {
                        if (!navigate) return;
                        if (source.kind === "project")
                          navigate({ name: "projects", projectId: source.id });
                        else if (source.kind === "task") navigate({ name: "tasks" });
                        else if (source.kind === "time_entry") navigate({ name: "entries" });
                      }}
                    >
                      {source.kind}: {source.label}
                    </button>
                  ))}
                </div>
              )}
              <AI_Prompt
                className="max-w-none py-0"
                defaultModel="Aevio Agent"
                headerAction=""
                headerText=""
                commands={[]}
                models={[
                  {
                    name: "Aevio Agent",
                    description: t("agent.localContextAndOnDeviceAi"),
                  },
                ]}
                onSubmit={send}
                placeholder={t("agent.askAevio")}
              />
            </CardContent>
          </Card>
        )}
        {tab === "routines" && (
          <section className="grid gap-4 lg:grid-cols-[1fr_1.4fr]">
            <Card className="border-line bg-surface">
              <CardHeader>
                <CardTitle>{t("agent.newRoutine")}</CardTitle>
                <CardDescription>Daily, weekly, monthly oder on-demand.</CardDescription>
              </CardHeader>
              <CardContent>
                <form className="space-y-3" onSubmit={createRoutine}>
                  <Input
                    value={name}
                    onChange={(event) => setName(event.target.value)}
                    placeholder={t("agent.name")}
                  />
                  <Input
                    value={prompt}
                    onChange={(event) => setPrompt(event.target.value)}
                    placeholder={t("agent.targetPrompt")}
                  />
                  <select
                    className="h-9 w-full rounded-md border border-line bg-surface px-3 text-sm text-fg"
                    value={cadence}
                    onChange={(event) => setCadence(event.target.value)}
                  >
                    <option value="daily">{t("agent.daily")}</option>
                    <option value="weekly">{t("agent.weekly")}</option>
                    <option value="monthly">{t("agent.monthly")}</option>
                    <option value="on-demand">{t("agent.onDemand")}</option>
                  </select>
                  <div className="flex flex-wrap gap-2" aria-label="Zustellung">
                    {(["in-app", "email", "slack"] as DeliveryChannel[]).map((channel) => (
                      <Button
                        key={channel}
                        type="button"
                        size="sm"
                        variant={delivery.includes(channel) ? "default" : "outline"}
                        onClick={() => toggleDelivery(channel)}
                      >
                        {channel}
                      </Button>
                    ))}
                  </div>
                  <Button type="submit" disabled={busy || !name.trim() || !prompt.trim()}>
                    {t("agent.saveRoutine")}
                  </Button>
                </form>
              </CardContent>
            </Card>
            <Card className="border-line bg-surface">
              <CardHeader>
                <CardTitle>{t("agent.routinesAndRuns")}</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                {snapshot?.routines.map((routine) => (
                  <div
                    key={routine.id}
                    className="flex items-center justify-between rounded-lg border border-line p-3"
                  >
                    <div>
                      <p className="font-medium text-fg">{routine.name}</p>
                      <p className="text-xs text-fg-muted">
                        {routine.cadence} · {routine.delivery.join(", ")}
                      </p>
                    </div>
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={busy}
                      onClick={() => void runRoutine(routine.id)}
                    >
                      {t("agent.runNow")}
                    </Button>
                  </div>
                ))}
                {snapshot?.routineRuns.slice(0, 8).map((run) => (
                  <div key={run.id} className="rounded-lg bg-surface-strong p-3 text-sm">
                    <span className="font-medium text-fg">
                      {t("agent.briefing")} · {run.status}
                    </span>
                    <span className="ml-2 text-fg-muted">{formatDate(run.createdAt)}</span>
                  </div>
                ))}
              </CardContent>
            </Card>
          </section>
        )}
        {tab === "reports" && (
          <section className="grid gap-4 lg:grid-cols-[1fr_1.4fr]">
            <Card className="border-line bg-surface">
              <CardHeader>
                <CardTitle>{t("agent.newReport")}</CardTitle>
                <CardDescription>
                  Der aktuelle Filter wird als historischer Run-Snapshot gespeichert.
                </CardDescription>
              </CardHeader>
              <CardContent>
                <form className="space-y-3" onSubmit={createReport}>
                  <Input
                    value={name}
                    onChange={(event) => setName(event.target.value)}
                    placeholder={t("agent.reportName")}
                  />
                  <select
                    className="h-9 w-full rounded-md border border-line bg-surface px-3 text-sm text-fg"
                    value={cadence}
                    onChange={(event) => setCadence(event.target.value)}
                  >
                    <option value="daily">{t("agent.daily")}</option>
                    <option value="weekly">{t("agent.weekly")}</option>
                    <option value="monthly">{t("agent.monthly")}</option>
                    <option value="on-demand">{t("agent.onDemand")}</option>
                  </select>
                  <div className="flex flex-wrap gap-2" aria-label="Zustellung">
                    {(["in-app", "email", "slack"] as DeliveryChannel[]).map((channel) => (
                      <Button
                        key={channel}
                        type="button"
                        size="sm"
                        variant={delivery.includes(channel) ? "default" : "outline"}
                        onClick={() => toggleDelivery(channel)}
                      >
                        {channel}
                      </Button>
                    ))}
                  </div>
                  <Button type="submit" disabled={busy || !name.trim()}>
                    {t("agent.saveReport")}
                  </Button>
                </form>
              </CardContent>
            </Card>
            <Card className="border-line bg-surface">
              <CardHeader>
                <CardTitle>{t("agent.reportsAndRuns")}</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                {snapshot?.reports.map((report) => (
                  <div
                    key={report.id}
                    className="flex items-center justify-between rounded-lg border border-line p-3"
                  >
                    <div>
                      <p className="font-medium text-fg">{report.name}</p>
                      <p className="text-xs text-fg-muted">
                        {report.cadence} · {report.format}
                      </p>
                    </div>
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={busy}
                      onClick={() => void runReport(report.id)}
                    >
                      {t("agent.runNow")}
                    </Button>
                  </div>
                ))}
                {snapshot?.reportRuns.slice(0, 8).map((run) => (
                  <div key={run.id} className="rounded-lg bg-surface-strong p-3 text-sm">
                    <span className="font-medium text-fg">
                      {t("agent.report")} · {run.output.entries as number} {t("agent.entries")} ·{" "}
                      {run.status}
                    </span>
                    <span className="ml-2 text-fg-muted">{formatDate(run.createdAt)}</span>
                  </div>
                ))}
              </CardContent>
            </Card>
          </section>
        )}
        {tab === "dashboards" && (
          <section className="space-y-4">
            <Card className="border-line bg-surface">
              <CardHeader>
                <CardTitle>{t("agent.createSavedFilter")}</CardTitle>
                <CardDescription>
                  Dashboards sind die dauerhaften Eingangsfilter für Report-Definitionen.
                </CardDescription>
              </CardHeader>
              <CardContent>
                <form className="flex gap-2" onSubmit={createDashboard}>
                  <Input
                    value={dashboardName}
                    onChange={(event) => setDashboardName(event.target.value)}
                    placeholder={t("agent.dashboardName")}
                  />
                  <Button type="submit" disabled={busy || !dashboardName.trim()}>
                    {t("agent.save")}
                  </Button>
                </form>
              </CardContent>
            </Card>
            <Card className="border-line bg-surface">
              <CardHeader>
                <CardTitle>{t("agent.dashboards")}</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                {snapshot?.dashboards.length === 0 && (
                  <p className="text-sm text-fg-muted">{t("agent.noSavedFilter")}</p>
                )}
                {snapshot?.dashboards.map((dashboard) => (
                  <div
                    key={dashboard.id}
                    className="flex items-center justify-between rounded-lg border border-line p-3"
                  >
                    <div>
                      <p className="font-medium text-fg">{dashboard.name}</p>
                      <p className="text-xs text-fg-muted">
                        {dashboard.pinned ? t("agent.favorite") : t("agent.saved")} ·{" "}
                        {formatDate(dashboard.updatedAt)}
                      </p>
                    </div>
                    <span className="text-xs text-fg-muted">{t("agent.filterReady")}</span>
                  </div>
                ))}
              </CardContent>
            </Card>
          </section>
        )}
      </div>
    </main>
  );
}
