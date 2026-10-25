import { invoke } from "./tauri";
import type { BreakEntry, BreakState } from "./breaks";
import type { LoginItemState, Settings, StoragePaths } from "./settings";
import type { Timer } from "./timers";

export interface WatchPairingInfo {
  enabled: boolean;
  port: number;
  token: string;
}
import type {
  ActivitySnapshot,
  UsageSnapshot,
  AiMetrics,
  AiStatus,
  AppRecord,
  BillableEntry,
  Category,
  Client,
  Label,
  NewLabel,
  EnergySummary,
  EntryDetail,
  EntryQuery,
  ExportResult,
  HintPreview,
  ImportSummary,
  IntegrationConnection,
  Invoice,
  InvoiceDraftInput,
  InvoiceProfile,
  InvoiceProfileInput,
  InvoiceSummary,
  NewClient,
  NewProject,
  NewTask,
  NewTimeEntry,
  PermissionInfo,
  Project,
  Task,
  ProjectRule,
  ProjectStats,
  ProjectSuggestion,
  RollupCell,
  RollupGroup,
  RuleSuggestion,
  TimeEntry,
  TimelineDevice,
  TimelineEvent,
  TimelineSyncState,
  TimelineSyncInput,
  TimelineSyncResult,
  PublishTimelineInput,
  UpdateClient,
  UpdateProject,
  UpdateLabel,
  UpdateTask,
  UpdateStatus,
  UpdateTimeEntry,
  CoachSnapshot,
  CoachTask,
  NewCoachTask,
  AutomationSnapshot,
  DailyWorkflowRun,
  DailyWorkflowSnapshot,
  AutomationRun,
  DeliveryChannel,
  ReportDefinition,
  Routine,
  SavedDashboard,
  DashboardShare,
  FinanceContract,
  FinanceRateCard,
  FinanceSnapshot,
  FinanceCapacityPlan,
  StaffingFreelancer,
  StaffingSnapshot,
  StaffingMatch,
  GovernanceSnapshot,
  GovernanceTeam,
  GovernanceTeamInput,
  GovernanceDepartment,
  GovernanceDepartmentInput,
  GovernanceMemberInput,
  WorkspaceDocument,
  WorkspaceNote,
  WorkspaceComment,
  WorkspaceAttachment,
  WorkspaceMention,
  WorkspaceCycle,
  WorkspaceInitiative,
  DecisionHealth,
  DecisionSettings,
  DecisionRequest,
  DecisionUseCaseInput,
  DecisionResult,
  MusicTrack,
  PluginCatalogEntry,
  PluginContributionSnapshot,
  PluginContributionPreference,
  PluginHealthSummary,
  PluginCommandResult,
  PluginSearchAggregate,
  PluginSearchResult,
  PluginJobSummary,
  PluginWorkflowRunSummary,
  PluginInstallationSummary,
  PluginInstallPlan,
  PluginRelease,
  PluginUpdateCandidate,
  PluginWorkspaceDesiredState,
  PluginWorkspaceDesiredStateInput,
  PluginUpdatePolicy,
} from "./types";

export interface AgentChatReply {
  answer: string;
  mode: "local-read-only" | "local-ai";
  context: {
    pendingEntries: number;
    activeProjects: number;
    categories: number;
    aiEngine: string;
    pendingPreview: string[];
    projectNames: string[];
    categoryNames: string[];
  };
}

export * from "./ipc/events";

export const PLUGINS_CHANGED = "plugins-changed";

/** Tauri rejects with a string; React errors are Error objects. Handle both. */
export function describeError(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}

export function listMusicTracks(): Promise<MusicTrack[]> {
  return invoke<MusicTrack[]>("list_music_tracks");
}

export function importMusicTrack(): Promise<MusicTrack | null> {
  return invoke<MusicTrack | null>("import_music_track");
}

// --- Plugin platform ----------------------------------------------------

export function pluginCatalog(): Promise<PluginCatalogEntry[]> {
  return invoke<PluginCatalogEntry[]>("plugin_catalog");
}

export function pluginRegistryCatalog(
  registryUrl: string,
  bearerToken: string,
): Promise<PluginCatalogEntry[]> {
  return invoke<PluginCatalogEntry[]>("plugin_registry_catalog", { registryUrl, bearerToken });
}

export function pluginRegistryUpdates(
  registryUrl: string,
  bearerToken: string,
): Promise<PluginUpdateCandidate[]> {
  return invoke<PluginUpdateCandidate[]>("plugin_registry_updates", {
    registryUrl,
    bearerToken,
  });
}

export function pluginWorkspacePluginStates(input: {
  registryUrl: string;
  bearerToken: string;
  workspaceId: string;
}): Promise<PluginWorkspaceDesiredState[]> {
  return invoke<PluginWorkspaceDesiredState[]>("plugin_workspace_plugin_states", {
    request: input,
  });
}

export function pluginSetWorkspacePluginState(input: {
  registryUrl: string;
  bearerToken: string;
  workspaceId: string;
  pluginId: string;
  input: PluginWorkspaceDesiredStateInput;
}): Promise<PluginWorkspaceDesiredState> {
  return invoke<PluginWorkspaceDesiredState>("plugin_set_workspace_plugin_state", {
    request: input,
  });
}

export function pluginInstalled(): Promise<PluginInstallationSummary[]> {
  return invoke<PluginInstallationSummary[]>("plugin_installed");
}

export function pluginContributions(): Promise<PluginContributionSnapshot[]> {
  return invoke<PluginContributionSnapshot[]>("plugin_contributions");
}

export function pluginLayout(): Promise<PluginContributionPreference[]> {
  return invoke<PluginContributionPreference[]>("plugin_layout");
}

export function pluginHealth(): Promise<PluginHealthSummary[]> {
  return invoke<PluginHealthSummary[]>("plugin_health");
}

export function pluginUpdatePolicy(pluginId: string): Promise<PluginUpdatePolicy> {
  return invoke<PluginUpdatePolicy>("plugin_update_policy", { pluginId });
}

export function pluginSetUpdatePolicy(policy: PluginUpdatePolicy): Promise<PluginUpdatePolicy> {
  return invoke<PluginUpdatePolicy>("plugin_set_update_policy", { policy });
}

export function pluginSetContributionPreference(
  preference: PluginContributionPreference,
): Promise<PluginContributionPreference> {
  return invoke<PluginContributionPreference>("plugin_set_contribution_preference", { preference });
}

export function pluginRestoreLayout(): Promise<void> {
  return invoke("plugin_restore_layout");
}

export function pluginPickPackage(): Promise<string | null> {
  return invoke<string | null>("plugin_pick_package");
}

export function pluginExecuteCommand(
  pluginId: string,
  commandId: string,
  input: Record<string, unknown> = {},
): Promise<PluginCommandResult> {
  return invoke<PluginCommandResult>("plugin_execute_command", {
    request: { pluginId, commandId, input },
  });
}

export function pluginOpenUi(pluginId: string): Promise<void> {
  return invoke("plugin_open_ui", { pluginId });
}

export function pluginSearch(
  pluginId: string,
  query: string,
  limit = 20,
): Promise<PluginSearchResult[]> {
  return invoke<PluginSearchResult[]>("plugin_search", { pluginId, query, limit });
}

export function pluginSearchAll(query: string, limit = 20): Promise<PluginSearchAggregate> {
  return invoke<PluginSearchAggregate>("plugin_search_all", { query, limit });
}

export function pluginPublishEvent(
  pluginId: string,
  event: string,
  payload: unknown,
): Promise<void> {
  return invoke("plugin_publish_event", { pluginId, event, payload });
}

export function pluginScheduleJob(input: {
  pluginId: string;
  jobId: string;
  intervalMs: number;
  payload?: unknown;
}): Promise<PluginJobSummary> {
  return invoke<PluginJobSummary>("plugin_schedule_job", {
    request: { ...input, payload: input.payload ?? {} },
  });
}

export function pluginCancelJob(pluginId: string, jobId: string): Promise<void> {
  return invoke("plugin_cancel_job", { pluginId, jobId });
}

export function pluginRunWorkflow(input: {
  pluginId: string;
  workflowId: string;
  trigger: string;
  idempotencyKey: string;
  input?: unknown;
}): Promise<PluginWorkflowRunSummary> {
  return invoke<PluginWorkflowRunSummary>("plugin_workflow_run", {
    request: { ...input, input: input.input ?? {} },
  });
}

export function pluginCancelWorkflow(
  pluginId: string,
  workflowId: string,
  idempotencyKey: string,
): Promise<void> {
  return invoke("plugin_workflow_cancel", { pluginId, workflowId, idempotencyKey });
}

export function pluginWorkflowHistory(pluginId: string): Promise<PluginWorkflowRunSummary[]> {
  return invoke<PluginWorkflowRunSummary[]>("plugin_workflow_history", { pluginId });
}

export function pluginPlanInstall(pluginId: string): Promise<PluginInstallPlan> {
  return invoke<PluginInstallPlan>("plugin_plan_install", { pluginId, scope: "personal" });
}

export function pluginInstall(
  pluginId: string,
  plan?: PluginInstallPlan,
): Promise<PluginInstallationSummary> {
  return invoke<PluginInstallationSummary>("plugin_install", {
    pluginId,
    scope: "personal",
    plan,
  });
}

export function pluginGrantPermissions(
  pluginId: string,
  permissions: string[],
): Promise<PluginInstallationSummary> {
  return invoke<PluginInstallationSummary>("plugin_grant_permissions", {
    pluginId,
    permissions,
  });
}

export function pluginRevokePermissions(
  pluginId: string,
  permissions: string[],
): Promise<PluginInstallationSummary> {
  return invoke<PluginInstallationSummary>("plugin_revoke_permissions", { pluginId, permissions });
}

export function pluginSetEnabled(
  pluginId: string,
  enabled: boolean,
): Promise<PluginInstallationSummary> {
  return invoke<PluginInstallationSummary>("plugin_set_enabled", { pluginId, enabled });
}

export async function pluginUninstall(pluginId: string): Promise<void> {
  await invoke("plugin_uninstall", { pluginId });
}

export async function pluginDeleteData(pluginId: string): Promise<void> {
  await invoke("plugin_delete_data", { pluginId });
}

export function pluginInstallPackage(
  packagePath: string,
  release: PluginRelease,
  permissions: string[] = [],
  enable = false,
): Promise<PluginInstallationSummary> {
  return invoke<PluginInstallationSummary>("plugin_install_package", {
    packagePath,
    release,
    scope: "personal",
    permissions,
    enable,
  });
}

export function pluginInstallRemote(input: {
  registryUrl: string;
  bearerToken: string;
  pluginId: string;
  release: PluginRelease;
  permissions?: string[];
  enable?: boolean;
}): Promise<PluginInstallationSummary> {
  return invoke<PluginInstallationSummary>("plugin_install_remote", {
    request: {
      ...input,
      scope: "personal",
      permissions: input.permissions ?? [],
      enable: input.enable ?? false,
    },
  });
}

export function pluginRollback(pluginId: string): Promise<PluginInstallationSummary> {
  return invoke<PluginInstallationSummary>("plugin_rollback", { pluginId });
}

export function pluginRecover(pluginId: string): Promise<PluginInstallationSummary> {
  return invoke<PluginInstallationSummary>("plugin_recover", { pluginId });
}

// --- Preferences ---

import { migrateSettings } from "./settings";

export async function getSettings(): Promise<Settings> {
  const raw = await invoke<Settings | Record<string, unknown>>("get_settings");
  return migrateSettings(raw as Record<string, unknown>);
}

export async function updateSettings(settings: Settings): Promise<Settings> {
  return await invoke<Settings>("update_settings", { settings });
}

export async function getDecisionSettings(): Promise<DecisionSettings> {
  return await invoke<DecisionSettings>("get_decision_settings");
}

export async function updateDecisionSettings(
  settings: DecisionSettings,
): Promise<DecisionSettings> {
  return await invoke<DecisionSettings>("update_decision_settings", { settings });
}

export async function decisionHealth(): Promise<DecisionHealth> {
  return await invoke<DecisionHealth>("decision_health");
}

export async function evaluateDecision(request: DecisionRequest): Promise<DecisionResult> {
  return await invoke<DecisionResult>("evaluate_decision", { request });
}

export async function evaluateDecisionUseCase(
  input: DecisionUseCaseInput,
): Promise<DecisionResult> {
  return await invoke<DecisionResult>("evaluate_decision_use_case", { input });
}

export async function evaluateCoachDecision(input: {
  kind: "focus_recovery" | "daily_review" | "task_prioritization";
  day: string;
  timeZone: string;
  options?: string[];
  recommended?: string;
}): Promise<DecisionResult> {
  return await invoke<DecisionResult>("coach_decision", { input });
}

export async function storagePaths(): Promise<StoragePaths> {
  return await invoke<StoragePaths>("storage_paths");
}

export async function timelineDay(
  startMs: number,
  endMs: number,
  workspaceId?: string,
): Promise<TimelineEvent[]> {
  return await invoke<TimelineEvent[]>("timeline_day", { startMs, endMs, workspaceId });
}

export async function publishTimelineSegment(input: PublishTimelineInput): Promise<TimelineEvent> {
  return await invoke<TimelineEvent>("publish_timeline_segment", { input });
}

export async function unpublishTimelineSegment(segmentId: number): Promise<void> {
  await invoke("unpublish_timeline_segment", { segmentId });
}

export async function listTimelineDevices(): Promise<TimelineDevice[]> {
  return await invoke<TimelineDevice[]>("list_timeline_devices");
}

export async function timelineSyncState(): Promise<TimelineSyncState> {
  return await invoke<TimelineSyncState>("timeline_sync_state");
}

export async function syncTimeline(input: TimelineSyncInput): Promise<TimelineSyncResult> {
  return await invoke<TimelineSyncResult>("sync_timeline", { input });
}

/** What macOS says about launching at login; it can change behind the app. */
export async function launchAtLogin(): Promise<LoginItemState> {
  return await invoke<LoginItemState>("launch_at_login");
}

export async function setLaunchAtLogin(enabled: boolean): Promise<LoginItemState> {
  return await invoke<LoginItemState>("set_launch_at_login", { enabled });
}

/** Reads authorization status of system permissions (Accessibility, Automation, etc.). */
export async function checkPermissions(): Promise<PermissionInfo[]> {
  return await invoke<PermissionInfo[]>("check_permissions");
}

export async function governanceSnapshot(): Promise<GovernanceSnapshot> {
  return await invoke<GovernanceSnapshot>("governance_snapshot", { actorId: "member-local" });
}
export async function saveGovernanceTeam(input: GovernanceTeamInput): Promise<GovernanceTeam> {
  return await invoke<GovernanceTeam>("save_governance_team", { actorId: "member-local", input });
}
export async function saveGovernanceDepartment(
  input: GovernanceDepartmentInput,
): Promise<GovernanceDepartment> {
  return await invoke<GovernanceDepartment>("save_governance_department", {
    actorId: "member-local",
    input,
  });
}
export async function saveGovernanceMember(input: GovernanceMemberInput): Promise<void> {
  await invoke("save_governance_member", { actorId: "member-local", input });
}
export async function saveGovernanceDefaults(
  defaults: Record<string, unknown>,
  applyToAll: boolean,
): Promise<void> {
  await invoke("save_governance_defaults", {
    actorId: "member-local",
    input: { defaults, applyToAll },
  });
}

// --- Productivity coach ---

export async function coachSnapshot(): Promise<CoachSnapshot> {
  return await invoke<CoachSnapshot>("coach_snapshot");
}

export async function listCoachTasks(): Promise<CoachTask[]> {
  return await invoke<CoachTask[]>("list_coach_tasks");
}

export async function createCoachTask(task: NewCoachTask): Promise<CoachTask> {
  return await invoke<CoachTask>("create_coach_task", { task });
}

export async function completeCoachTask(id: string): Promise<CoachTask> {
  return await invoke<CoachTask>("complete_coach_task", { id });
}

export async function deleteCoachTask(id: string): Promise<void> {
  await invoke("delete_coach_task", { id });
}

// --- Pulse panel ---

/** Resizes the always-visible widget to its compact or expanded layout. */
export async function resizeWidget(height: number): Promise<void> {
  await invoke("resize_widget", { height });
}

/** Sizes the menu-bar panel to its content, keeping it under the icon. */
export async function resizePulsePanel(height: number): Promise<void> {
  await invoke("resize_pulse_panel", { height });
}

export async function hidePulsePanel(): Promise<void> {
  await invoke("hide_pulse_panel");
}

export async function openProductivitySummary(): Promise<void> {
  await invoke("open_productivity_summary");
}

export async function hideProductivitySummary(): Promise<void> {
  await invoke("hide_productivity_summary");
}

/** Closes the panel and brings the main window forward. */
export async function openMainWindow(review: boolean, timerId?: string): Promise<void> {
  await invoke("open_main_window", { review, timerId });
}

/**
 * Undoes (or redoes) the last change to entries or the catalog. Resolves to
 * what it was ("Split", "Accept", ...), or null when there was nothing.
 */
export async function undo(redo = false): Promise<string | null> {
  return await invoke<string | null>("undo", { redo });
}

// --- Break reminders ---

export async function breakState(): Promise<BreakState> {
  return await invoke<BreakState>("break_state");
}

export async function dismissStopwatchReminder(id: string, startedAt: number): Promise<BreakState> {
  return await invoke<BreakState>("dismiss_stopwatch_reminder", {
    id,
    startedAt,
  });
}

/** Starts the pending reminder's break, or a manual one when none is pending. */
export async function startBreak(): Promise<BreakState> {
  return await invoke<BreakState>("start_break");
}

export async function endBreak(): Promise<BreakState> {
  return await invoke<BreakState>("end_break");
}

export async function snoozeBreak(minutes: number): Promise<BreakState> {
  return await invoke<BreakState>("snooze_break", { minutes });
}

export async function skipBreak(): Promise<BreakState> {
  return await invoke<BreakState>("skip_break");
}

/** Re-expands a reminder that collapsed to the corner capsule. */
export async function expandBreakReminder(): Promise<BreakState> {
  return await invoke<BreakState>("expand_break_reminder");
}

/** Adds five minutes to the running break. */
export async function extendBreak(): Promise<BreakState> {
  return await invoke<BreakState>("extend_break");
}

/** Silences reminders until `until` (epoch ms); null turns them back on. */
export async function pauseBreakReminders(until: number | null): Promise<BreakState> {
  return await invoke<BreakState>("pause_break_reminders", { until });
}

export async function listBreaks(sinceMs: number, untilMs: number): Promise<BreakEntry[]> {
  return await invoke<BreakEntry[]>("list_breaks", { sinceMs, untilMs });
}

/** Sizes the top-right reminder panel to its card. */
export async function resizeReminderPanel(width: number, height: number): Promise<void> {
  await invoke("resize_reminder_panel", { width, height });
}

/** Brings the main window forward on Settings > Notifications. */
export async function openBreakSettings(): Promise<void> {
  await invoke("open_break_settings");
}

/** Dev builds only: raises a sample reminder. */
export async function devSampleBreakReminder(): Promise<BreakState> {
  return await invoke<BreakState>("dev_sample_break_reminder");
}

export async function previewBreakChime(): Promise<void> {
  await invoke("preview_break_chime");
}

// --- Activity & Capture ---

export async function fetchActivitySnapshot(sinceMs: number): Promise<ActivitySnapshot> {
  return await invoke<ActivitySnapshot>("activity_snapshot", { sinceMs });
}

export async function fetchUsageSnapshot(sinceMs: number): Promise<UsageSnapshot> {
  return await invoke<UsageSnapshot>("usage_snapshot", { sinceMs });
}

export async function setCaptureEnabled(enabled: boolean): Promise<void> {
  await invoke("set_capture_enabled", { enabled });
}

/** Starts an explicit activity session owned by the Rust activity store. */
export async function startSession(
  kind: "focus" | "meeting" | "break",
  label?: string,
): Promise<void> {
  await invoke("start_session", { kind, label });
}

/** Stops the current explicit activity session, if one is running. */
export async function stopSession(): Promise<void> {
  await invoke("stop_session");
}

// --- Categories ---

export async function listCategories(): Promise<Category[]> {
  return await invoke<Category[]>("list_categories");
}

export async function deleteCategory(id: string): Promise<void> {
  await invoke("delete_category", { id });
}

// --- Projects ---

export async function listProjects(): Promise<Project[]> {
  return await invoke<Project[]>("list_projects");
}

export async function createProject(project: NewProject): Promise<Project> {
  return await invoke<Project>("create_project", { project });
}

export async function updateProject(id: string, patch: UpdateProject): Promise<Project> {
  return await invoke<Project>("update_project", { id, patch });
}

export async function deleteProject(id: string): Promise<void> {
  await invoke("delete_project", { id });
}

// --- Workspace content -------------------------------------------------

export async function listDocuments(): Promise<WorkspaceDocument[]> {
  return await invoke<WorkspaceDocument[]>("list_documents");
}

export async function createDocument(input: {
  title: string;
  content: string;
}): Promise<WorkspaceDocument> {
  return await invoke<WorkspaceDocument>("create_document", { input });
}

export async function updateDocument(
  id: string,
  patch: { title?: string; content?: string },
): Promise<WorkspaceDocument> {
  return await invoke<WorkspaceDocument>("update_document", { id, patch });
}

export async function deleteDocument(id: string): Promise<void> {
  await invoke("delete_document", { id });
}

export async function listNotes(): Promise<WorkspaceNote[]> {
  return await invoke<WorkspaceNote[]>("list_notes");
}

export async function createNote(input: { title: string; body: string }): Promise<WorkspaceNote> {
  return await invoke<WorkspaceNote>("create_note", { input });
}

export async function updateNote(
  id: string,
  patch: { title?: string; body?: string },
): Promise<WorkspaceNote> {
  return await invoke<WorkspaceNote>("update_note", { id, patch });
}

export async function deleteNote(id: string): Promise<void> {
  await invoke("delete_note", { id });
}

export async function listComments(
  targetType: string,
  targetId: string,
): Promise<WorkspaceComment[]> {
  return await invoke<WorkspaceComment[]>("list_comments", { targetType, targetId });
}

export async function createComment(input: {
  targetType: string;
  targetId: string;
  body: string;
  author?: string;
}): Promise<WorkspaceComment> {
  return await invoke<WorkspaceComment>("create_comment", { input });
}

export async function listAttachments(
  targetType: string,
  targetId: string,
): Promise<WorkspaceAttachment[]> {
  return await invoke<WorkspaceAttachment[]>("list_attachments", { targetType, targetId });
}

export async function createAttachment(input: {
  targetType: string;
  targetId: string;
  name: string;
  path: string;
  mimeType?: string;
  size?: number;
}): Promise<WorkspaceAttachment> {
  return await invoke<WorkspaceAttachment>("create_attachment", { input });
}

export async function listMentions(
  targetType: string,
  targetId: string,
): Promise<WorkspaceMention[]> {
  return await invoke<WorkspaceMention[]>("list_mentions", { targetType, targetId });
}

export async function listCycles(): Promise<WorkspaceCycle[]> {
  return await invoke<WorkspaceCycle[]>("list_cycles");
}

export async function createCycle(input: {
  name: string;
  goal: string;
  startsAt: number;
  endsAt: number;
}): Promise<WorkspaceCycle> {
  return await invoke<WorkspaceCycle>("create_cycle", { input });
}

export async function listInitiatives(): Promise<WorkspaceInitiative[]> {
  return await invoke<WorkspaceInitiative[]>("list_initiatives");
}

export async function createInitiative(input: {
  name: string;
  description: string;
  targetDate?: number;
  projectIds: string[];
}): Promise<WorkspaceInitiative> {
  return await invoke<WorkspaceInitiative>("create_initiative", { input });
}

// --- Tasks and labels ---------------------------------------------------

export async function listTasks(projectId?: string): Promise<Task[]> {
  return await invoke<Task[]>("list_tasks", { projectId });
}

export async function createTask(task: NewTask): Promise<Task> {
  return await invoke<Task>("create_task", { task });
}

export async function updateTask(id: string, patch: UpdateTask): Promise<Task> {
  return await invoke<Task>("update_task", { id, patch });
}

export async function deleteTask(id: string): Promise<void> {
  await invoke("delete_task", { id });
}

export async function listLabels(): Promise<Label[]> {
  return await invoke<Label[]>("list_labels");
}

export async function createLabel(label: NewLabel): Promise<Label> {
  return await invoke<Label>("create_label", { label });
}

export async function updateLabel(id: string, patch: UpdateLabel): Promise<Label> {
  return await invoke<Label>("update_label", { id, patch });
}

export async function deleteLabel(id: string): Promise<void> {
  await invoke("delete_label", { id });
}

export async function listIntegrations(): Promise<IntegrationConnection[]> {
  return await invoke<IntegrationConnection[]>("list_integrations");
}

export async function saveIntegration(
  connection: IntegrationConnection,
): Promise<IntegrationConnection> {
  return await invoke<IntegrationConnection>("save_integration", { connection });
}

export async function projectStats(
  rangeStart: number,
  rangeEnd: number,
  monthStart: number,
): Promise<ProjectStats[]> {
  return await invoke<ProjectStats[]>("project_stats", {
    rangeStart,
    rangeEnd,
    monthStart,
  });
}

export async function projectRules(projectId: string): Promise<ProjectRule[]> {
  return await invoke<ProjectRule[]>("project_rules", { projectId });
}

export async function previewProjectHints(hints: string, sinceMs: number): Promise<HintPreview> {
  return await invoke<HintPreview>("preview_project_hints", { hints, sinceMs });
}

export async function discoverProjects(sinceMs: number): Promise<ProjectSuggestion[]> {
  return await invoke<ProjectSuggestion[]>("discover_projects", { sinceMs });
}

export async function dismissProjectSuggestion(key: string): Promise<void> {
  await invoke("dismiss_project_suggestion", { key });
}

export async function importProjectsCsv(text: string): Promise<ImportSummary> {
  return await invoke<ImportSummary>("import_projects_csv", { text });
}

// --- Clients ---

export async function listClients(): Promise<Client[]> {
  return await invoke<Client[]>("list_clients");
}

export async function createClient(client: NewClient): Promise<Client> {
  return await invoke<Client>("create_client", { client });
}

export async function updateClient(id: string, patch: UpdateClient): Promise<Client> {
  return await invoke<Client>("update_client", { id, patch });
}

export async function deleteClient(id: string): Promise<void> {
  await invoke("delete_client", { id });
}

// --- Invoices ---

export async function listInvoices(): Promise<InvoiceSummary[]> {
  return await invoke<InvoiceSummary[]>("list_invoices");
}

export async function getInvoice(id: string): Promise<Invoice> {
  return await invoke<Invoice>("get_invoice", { id });
}

export async function listBillableEntries(
  clientId: string,
  startMs: number,
  endMs: number,
  invoiceId?: string,
): Promise<BillableEntry[]> {
  return await invoke<BillableEntry[]>("list_billable_entries", {
    clientId,
    startMs,
    endMs,
    invoiceId: invoiceId ?? null,
  });
}

/** The draft priced and validated by Rust, without storing it. */
export async function quoteInvoice(draft: InvoiceDraftInput): Promise<Invoice> {
  return await invoke<Invoice>("quote_invoice", { draft });
}

/** The DRAFT-watermarked PDF for a draft, straight from the Rust renderer. */
export async function renderInvoicePreview(draft: InvoiceDraftInput): Promise<Uint8Array> {
  const bytes = await invoke<ArrayBuffer>("render_invoice_preview", { draft });
  return new Uint8Array(bytes);
}

/** The archived PDF of a finalized invoice. */
export async function getInvoicePdf(id: string): Promise<Uint8Array> {
  const bytes = await invoke<ArrayBuffer>("get_invoice_pdf", { id });
  return new Uint8Array(bytes);
}

export async function saveInvoiceDraft(draft: InvoiceDraftInput): Promise<Invoice> {
  return await invoke<Invoice>("save_invoice_draft", { draft });
}

export async function finalizeInvoice(id: string): Promise<Invoice> {
  return await invoke<Invoice>("finalize_invoice", { id });
}

export async function setInvoicePaid(id: string, paid: boolean): Promise<void> {
  await invoke("set_invoice_paid", { id, paid });
}

export async function voidInvoice(id: string): Promise<void> {
  await invoke("void_invoice", { id });
}

export async function deleteDraftInvoice(id: string): Promise<void> {
  await invoke("delete_draft_invoice", { id });
}

export async function deleteVoidInvoice(id: string): Promise<void> {
  await invoke("delete_void_invoice", { id });
}

/**
 * Asks where to save an invoice PDF. Pass `id` for a finalized invoice or
 * `draft` for a watermarked draft render. Resolves to the saved path, or null
 * when the user cancels.
 */
export async function exportInvoicePdf(
  target: { id: string } | { draft: InvoiceDraftInput },
): Promise<string | null> {
  return await invoke<string | null>("export_invoice_pdf", {
    id: "id" in target ? target.id : null,
    draft: "draft" in target ? target.draft : null,
  });
}

export async function getInvoiceProfile(): Promise<InvoiceProfile> {
  return await invoke<InvoiceProfile>("get_invoice_profile");
}

export async function updateInvoiceProfile(profile: InvoiceProfileInput): Promise<InvoiceProfile> {
  return await invoke<InvoiceProfile>("update_invoice_profile", { profile });
}

/** The stored logo (PNG), or an empty array when none is set. */
export async function getInvoiceLogo(): Promise<Uint8Array> {
  return new Uint8Array(await invoke<ArrayBuffer>("get_invoice_logo"));
}

/** Sends the image bytes as the raw request body; Rust validates and resizes. */
export async function setInvoiceLogo(image: Uint8Array): Promise<void> {
  await invoke("set_invoice_logo", image);
}

export async function clearInvoiceLogo(): Promise<void> {
  await invoke("clear_invoice_logo");
}

// --- Time Entries ---

export async function listTimeEntries(startMs: number, endMs: number): Promise<TimeEntry[]> {
  return await invoke<TimeEntry[]>("list_time_entries", { startMs, endMs });
}

export async function getEntryDetail(id: string): Promise<EntryDetail> {
  return await invoke<EntryDetail>("get_entry_detail", { id });
}

export async function updateTimeEntry(id: string, patch: UpdateTimeEntry): Promise<TimeEntry> {
  return await invoke<TimeEntry>("update_time_entry", { id, patch });
}

export async function approveTimeEntries(ids: string[]): Promise<TimeEntry[]> {
  return await invoke<TimeEntry[]>("approve_time_entries", { ids });
}

/** Returns approved entries to pending for editing; no AI re-run. */
export async function unapproveTimeEntries(ids: string[]): Promise<TimeEntry[]> {
  return await invoke<TimeEntry[]>("unapprove_time_entries", { ids });
}

/** One patch applied to many entries; returns them updated. */
export async function updateTimeEntries(
  ids: string[],
  patch: UpdateTimeEntry,
): Promise<TimeEntry[]> {
  return await invoke<TimeEntry[]>("update_time_entries", { ids, patch });
}

// --- Reports ---

/** Entries matching a filter, newest first. */
export async function queryTimeEntries(filter: EntryQuery, limit?: number): Promise<TimeEntry[]> {
  return await invoke<TimeEntry[]>("query_time_entries", { filter, limit });
}

/**
 * Totals per group per bucket, summed in SQL. `boundaries` holds n + 1
 * ascending local-time edges for n buckets.
 */
export async function entryRollup(
  filter: EntryQuery,
  boundaries: number[],
  groupBy: RollupGroup,
): Promise<RollupCell[]> {
  return await invoke<RollupCell[]>("entry_rollup", {
    filter,
    boundaries,
    groupBy,
  });
}

/** Writes the filtered entries to Downloads; returns the file. */
export async function exportTimeEntries(
  filter: EntryQuery,
  format: "csv" | "json",
): Promise<ExportResult> {
  return await invoke<ExportResult>("export_time_entries", { filter, format });
}

export async function rejectTimeEntry(id: string): Promise<void> {
  await invoke("reject_time_entry", { id });
}

export async function splitTimeEntry(id: string, atMs: number): Promise<[TimeEntry, TimeEntry]> {
  return await invoke<[TimeEntry, TimeEntry]>("split_time_entry", {
    id,
    atMs,
  });
}

export async function mergeTimeEntries(primaryId: string, secondaryId: string): Promise<TimeEntry> {
  return await invoke<TimeEntry>("merge_time_entries", {
    primaryId,
    secondaryId,
  });
}

export async function deleteTimeEntry(id: string): Promise<void> {
  await invoke("delete_time_entry", { id });
}

export async function deleteTimeEntries(ids: string[]): Promise<void> {
  await invoke("delete_time_entries", { ids });
}

export async function createTimeEntry(entry: NewTimeEntry): Promise<TimeEntry> {
  return await invoke<TimeEntry>("create_time_entry", { entry });
}

export async function rebuildTimeEntries(startMs: number, endMs: number): Promise<TimeEntry[]> {
  return await invoke<TimeEntry[]>("rebuild_time_entries", { startMs, endMs });
}

// --- AI suggestions ---

export async function aiStatus(): Promise<AiStatus> {
  return await invoke<AiStatus>("ai_status");
}

export async function agentChat(message: string): Promise<AgentChatReply> {
  return await invoke<AgentChatReply>("agent_chat", { message });
}

export interface WorkSourceRef {
  kind: string;
  id: string;
  label: string;
}
export interface WorkAnswer {
  answer: string;
  mode: string;
  rangeStart: number;
  rangeEnd: number;
  projectId: string | null;
  projectName: string | null;
  totalMs: number;
  entries: unknown[];
  tasks: unknown[];
  evidence: string[];
  sources: WorkSourceRef[];
  context: {
    pendingEntries: number;
    activeProjects: number;
    aiEngine: string;
    projectNames: string[];
  };
}

export async function workAsk(question: string): Promise<WorkAnswer> {
  return await invoke<WorkAnswer>("work_ask", { question });
}

export async function workRelationshipsList(): Promise<unknown[]> {
  return await invoke<unknown[]>("work_relationships_list");
}

export async function workRelationshipAdd(
  fromType: string,
  fromId: string,
  relation: string,
  toType: string,
  toId: string,
): Promise<unknown> {
  return await invoke("work_relationship_add", { fromType, fromId, relation, toType, toId });
}

export async function workTraverse(
  startType: string,
  startId: string,
  maxDepth?: number,
  maxResults?: number,
): Promise<unknown> {
  return await invoke("work_traverse", { startType, startId, maxDepth, maxResults });
}

export async function workProjectSuggestions(sinceMs: number): Promise<unknown[]> {
  return await invoke<unknown[]>("work_project_suggestions", { sinceMs });
}

export async function workDismissSuggestion(key: string): Promise<void> {
  return await invoke<void>("work_dismiss_suggestion", { key });
}

export async function workMatchExisting(key: string, projectId: string): Promise<void> {
  return await invoke<void>("work_match_existing", { key, projectId });
}

export async function automationSnapshot(): Promise<AutomationSnapshot> {
  return await invoke<AutomationSnapshot>("automation_snapshot");
}

export async function dailyWorkflowSnapshot(): Promise<DailyWorkflowSnapshot> {
  return await invoke<DailyWorkflowSnapshot>("daily_workflow_snapshot");
}

export async function runDailyWorkflow(): Promise<DailyWorkflowRun> {
  return await invoke<DailyWorkflowRun>("run_daily_workflow");
}

export async function createRoutine(input: {
  name: string;
  prompt: string;
  cadence?: string;
  sendAt?: string;
  delivery?: DeliveryChannel[];
}): Promise<Routine> {
  return await invoke<Routine>("create_routine", { input });
}

export async function createReport(input: {
  name: string;
  filter: Record<string, unknown>;
  cadence?: string;
  delivery?: DeliveryChannel[];
}): Promise<ReportDefinition> {
  return await invoke<ReportDefinition>("create_report", { input });
}

export async function createDashboard(input: {
  name: string;
  filter: Record<string, unknown>;
}): Promise<SavedDashboard> {
  return await invoke<SavedDashboard>("create_dashboard", { input });
}

export async function runRoutine(id: string): Promise<AutomationRun> {
  return await invoke<AutomationRun>("run_routine", { id });
}

export async function runReport(id: string): Promise<AutomationRun> {
  return await invoke<AutomationRun>("run_report", { id });
}

// --- Finance -----------------------------------------------------------

export async function financeSnapshot(startMs: number, endMs: number): Promise<FinanceSnapshot> {
  return await invoke<FinanceSnapshot>("finance_snapshot", { filter: { startMs, endMs } });
}

export async function listFinanceRateCards(): Promise<FinanceRateCard[]> {
  return await invoke<FinanceRateCard[]>("list_finance_rate_cards");
}

export async function saveFinanceRateCard(input: {
  projectId: string;
  billRateCents: number;
  costRateCents: number;
  currency: string;
  effectiveFrom?: number;
}): Promise<FinanceRateCard> {
  return await invoke<FinanceRateCard>("save_finance_rate_card", { input });
}

export async function listFinanceContracts(): Promise<FinanceContract[]> {
  return await invoke<FinanceContract[]>("list_finance_contracts");
}

export async function saveFinanceContract(
  input: Omit<FinanceContract, "id"> & { id?: string },
): Promise<FinanceContract> {
  return await invoke<FinanceContract>("save_finance_contract", { input });
}

export async function shareFinanceDashboard(
  dashboardId: string,
  expiresAt?: number,
): Promise<DashboardShare> {
  return await invoke<DashboardShare>("share_finance_dashboard", {
    dashboardId,
    expiresAt: expiresAt ?? null,
  });
}

export async function listFinanceCapacityPlans(): Promise<FinanceCapacityPlan[]> {
  return await invoke<FinanceCapacityPlan[]>("list_finance_capacity_plans");
}

export async function saveFinanceCapacityPlan(
  input: Omit<FinanceCapacityPlan, "id"> & { id?: string },
): Promise<FinanceCapacityPlan> {
  return await invoke<FinanceCapacityPlan>("save_finance_capacity_plan", { input });
}

export async function listFinanceDashboardShares(dashboardId: string): Promise<DashboardShare[]> {
  return await invoke<DashboardShare[]>("list_finance_dashboard_shares", { dashboardId });
}

export async function revokeFinanceDashboardShare(id: string): Promise<void> {
  await invoke("revoke_finance_dashboard_share", { id });
}

// --- Staffing and freelancers ----------------------------------------

export async function staffingSnapshot(): Promise<StaffingSnapshot> {
  return await invoke<StaffingSnapshot>("staffing_snapshot");
}

export async function listStaffingFreelancers(): Promise<StaffingFreelancer[]> {
  return await invoke<StaffingFreelancer[]>("list_staffing_freelancers");
}

export async function saveStaffingFreelancer(
  input: Omit<StaffingFreelancer, "id" | "createdAt" | "updatedAt"> & { id?: string },
): Promise<StaffingFreelancer> {
  return await invoke<StaffingFreelancer>("save_staffing_freelancer", { input });
}

export async function saveStaffingRecord(
  kind: string,
  payload: Record<string, unknown>,
): Promise<void> {
  await invoke("save_staffing_record", { kind, payload });
}

export async function staffingMatches(requestId: string): Promise<StaffingMatch[]> {
  return await invoke<StaffingMatch[]>("staffing_matches", { requestId });
}

export async function retryClassification(id: string): Promise<void> {
  await invoke("retry_classification", { id });
}

export async function resolveRuleSuggestion(
  suggestion: RuleSuggestion,
  accept: boolean,
): Promise<void> {
  await invoke("resolve_rule_suggestion", { suggestion, accept });
}

// --- Learning loop ---

export async function aiMetrics(days: number): Promise<AiMetrics> {
  return await invoke<AiMetrics>("ai_metrics", { days });
}

/** Queues a retrain that skips the idle/power gate; returns the new status. */
export async function aiRetrain(): Promise<AiStatus> {
  return await invoke<AiStatus>("ai_retrain");
}

export async function aiResetLearned(days: number): Promise<AiMetrics> {
  return await invoke<AiMetrics>("ai_reset_learned", { days });
}

// --- Apps ---

export async function listApps(): Promise<AppRecord[]> {
  return await invoke<AppRecord[]>("list_apps");
}

export async function updateApp(
  id: string,
  defaultCategoryId?: string,
  defaultProjectId?: string,
  excluded?: boolean,
): Promise<AppRecord> {
  return await invoke<AppRecord>("update_app", {
    id,
    defaultCategoryId,
    defaultProjectId,
    excluded,
  });
}

// --- Timers (Legacy/Compat) ---

export function listTimers(): Promise<Timer[]> {
  return invoke<Timer[]>("list_timers");
}

/** Returns the local pairing material for the native watch companion. */
export function watchPairing(): Promise<WatchPairingInfo> {
  return invoke<WatchPairingInfo>("watch_pairing");
}

export function createTimer(label: string): Promise<Timer[]> {
  return invoke<Timer[]>("create_timer", { label });
}

export function startTimer(id: string): Promise<Timer[]> {
  return invoke<Timer[]>("start_timer", { id });
}

export function pauseTimer(id: string): Promise<Timer[]> {
  return invoke<Timer[]>("pause_timer", { id });
}

export function resetTimer(id: string): Promise<Timer[]> {
  return invoke<Timer[]>("reset_timer", { id });
}

export function renameTimer(id: string, label: string): Promise<Timer[]> {
  return invoke<Timer[]>("rename_timer", { id, label });
}

export function addTimerSegment(id: string, durationMs: number): Promise<Timer[]> {
  return invoke<Timer[]>("add_timer_segment", { id, durationMs });
}

export function updateTimerSegment(
  id: string,
  segmentId: number,
  durationMs: number,
): Promise<Timer[]> {
  return invoke<Timer[]>("update_timer_segment", {
    id,
    segmentId,
    durationMs,
  });
}

export function deleteTimerSegment(id: string, segmentId: number): Promise<Timer[]> {
  return invoke<Timer[]>("delete_timer_segment", { id, segmentId });
}

export function deleteTimer(id: string): Promise<Timer[]> {
  return invoke<Timer[]>("delete_timer", { id });
}

// --- Battery & Energy ---

export function getEnergySummary(days?: number): Promise<EnergySummary> {
  return invoke<EnergySummary>("get_energy_summary", { days });
}

export function resetEnergyHistory(): Promise<EnergySummary> {
  return invoke<EnergySummary>("reset_energy_history");
}

// --- Updates ---

export function updateStatus(): Promise<UpdateStatus> {
  return invoke<UpdateStatus>("update_status");
}

export function checkForUpdates(): Promise<UpdateStatus> {
  return invoke<UpdateStatus>("check_for_updates");
}

/** Resolves only if the install fails to restart; otherwise the app relaunches. */
export function installUpdate(): Promise<void> {
  return invoke<void>("install_update");
}
