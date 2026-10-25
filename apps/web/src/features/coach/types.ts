export interface Plan {
  day: string;
  timeZone: string;
  goal: string;
  notes: string;
  availableMinutes: number;
  energy: number | null;
  state: "planned" | "skipped";
  revision: number;
}
export type Outcome =
  | "achieved"
  | "partially_achieved"
  | "not_achieved"
  | "changed"
  | "not_reviewed";
export interface Review {
  outcome: Outcome;
  worked: string;
  obstacles: string;
  tomorrow: string;
  revision: number;
}
export type Style = "minimal" | "calm" | "supportive" | "structured" | "direct";
export interface Preferences {
  enabled: boolean;
  morningPlanning: boolean;
  reminders: boolean;
  automaticHints: boolean;
  aiEnabled: boolean;
  memoryEnabled: boolean;
  style: Style;
  quietStart: number;
  quietEnd: number;
  cooldownMinutes: number;
  reviewMinute: number;
  visualTimer: boolean;
  widgetContext: boolean;
  widgetVisible: boolean;
  planningBufferMinutes: number;
  personalIntelligenceEnabled: boolean;
}
export interface TaskRef {
  source: "personal" | "project";
  id: string;
}
export interface CatalogTask {
  reminderAt: number | null;
  task: TaskRef;
  title: string;
  notes: string;
  done: boolean;
  priority: number;
  estimateMinutes: number | null;
}
export interface Step {
  title: string;
  done: boolean;
}
export interface PlannedTask extends CatalogTask {
  id: string;
  important: boolean;
  estimateMinutes: number;
  position: number;
  steps: Step[];
  actualMs: number;
  postponements: number;
}
export interface Day {
  day: string;
  timeZone: string;
  plan: Plan | null;
  review: Review;
  tasks: PlannedTask[];
  catalog: CatalogTask[];
  carryover: PlannedTask[];
  preferences: Preferences;
  trackedMs: number;
  unassignedMs: number;
  plannedMinutes: number;
  overbookedMinutes: number;
  changes: number;
  timeEntries: { id: string; description: string; durationMs: number; assignedTo: string | null }[];
}
export type CoachCommand =
  | { kind: "save_plan"; plan: Plan }
  | { kind: "save_review"; review: Review; revision: number }
  | {
      kind: "create_task";
      reminderAt: number | null;
      title: string;
      notes: string;
      priority: number;
      estimateMinutes: number;
      important: boolean;
    }
  | { kind: "add_task"; task: TaskRef; estimateMinutes: number; important: boolean }
  | {
      kind: "edit_task";
      reminderAt: number | null;
      id: string;
      title: string;
      notes: string;
      priority: number;
      estimateMinutes: number;
      important: boolean;
    }
  | { kind: "complete_task"; id: string; done: boolean }
  | { kind: "move_task"; id: string; targetDay: string }
  | { kind: "remove_task"; id: string }
  | { kind: "reorder"; ids: string[] }
  | { kind: "save_steps"; id: string; steps: Step[] }
  | { kind: "link_time"; id: string; entryId: string }
  | { kind: "unlink_time"; entryId: string }
  | { kind: "save_preferences"; preferences: Preferences }
  | { kind: "erase_coach" };
export type StartingReason =
  | "cannot_start"
  | "unsure"
  | "too_large"
  | "low_energy"
  | "distracted"
  | "first_step";
export interface AssistRequest {
  title: string;
  reason: StartingReason;
  granularity: number;
  estimateMinutes: number;
  previousSteps: string[];
  useAi: boolean;
}
export interface Proposal {
  steps: string[];
  nextStep: string;
  source: "rules" | "on_device";
  note: string | null;
  promptVersion: string;
}
export type Apply = (command: CoachCommand) => Promise<Day | null>;
export interface InterventionAction {
  id: string;
  label: string;
  kind: string;
}
export interface Intervention {
  id: string;
  kind: string;
  priority: number;
  title: string;
  message: string;
  actions: InterventionAction[];
}
export interface Observation {
  id: string;
  kind: string;
  explanation: string;
  samples: number;
}
export interface Recommendation {
  id: string;
  kind: string;
  title: string;
  message: string;
  evidence: string;
}
export interface PersonalIntelligence {
  observations: Observation[];
  recommendations: Recommendation[];
}
