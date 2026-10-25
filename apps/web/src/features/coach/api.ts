import { invoke } from "@/lib/tauri";
import type {
  AssistRequest,
  CoachCommand,
  Day,
  Intervention,
  PersonalIntelligence,
  Proposal,
} from "./types";
export const fetchDay = (day: string, timeZone: string) =>
  invoke<Day>("coach_day", { day, timeZone });
export const apply = (day: string, timeZone: string, command: CoachCommand) =>
  invoke<Day>("coach_apply", { day, timeZone, command });
export const propose = (request: AssistRequest) => invoke<Proposal>("coach_propose", { request });
export const fetchIntervention = (day: string, timeZone: string) =>
  invoke<Intervention | null>("coach_intervention", { day, timeZone });
export const dismissIntervention = (day: string, kind: string) =>
  invoke<void>("coach_dismiss_intervention", { day, kind });
export const fetchPersonalIntelligence = (day: string, timeZone: string) =>
  invoke<PersonalIntelligence>("coach_intelligence", { day, timeZone });
export const applyRecommendation = (day: string, recommendationId: string, kind: string) =>
  invoke<void>("coach_apply_recommendation", { day, recommendationId, kind });
export const suppressRecommendation = (day: string, recommendationId: string) =>
  invoke<void>("coach_suppress_recommendation", { day, recommendationId });
export const suppressObservation = (day: string, observationId: string) =>
  invoke<void>("coach_suppress_observation", { day, observationId });
export const resetPersonalIntelligence = () =>
  invoke<void>("coach_reset_personal_intelligence", {});
export const snoozeIntervention = (day: string, kind: string, minutes: number) =>
  invoke<void>("coach_snooze_intervention", { day, kind, minutes });

export const fetchFocus = <T>() => invoke<T | null>("coach_focus");

export const runFocusAction = <T>(
  command: "coach_start_focus" | "coach_end_focus",
  args: Record<string, unknown>,
) => invoke<T | null>(command, args);

export const fetchPlanProposal = <T>(day: string, timeZone: string, useAi: boolean) =>
  invoke<T>("coach_plan_proposal", { day, timeZone, useAi });

export const startFocus = (input: {
  day: string;
  timeZone: string;
  id: string;
  minutes: number;
  nextStep: string;
  bodyDoubling: boolean;
}) => invoke<void>("coach_start_focus", input);

export const fetchBrain = <T>() => invoke<T>("coach_brain");

export const applyBrain = <T, C>(day: string, timeZone: string, command: C) =>
  invoke<T>("coach_brain_apply", { day, timeZone, command });

export const classifyThought = <T>(content: string) =>
  invoke<T>("coach_classify_thought", { content });

export const fetchInsights = <T>(day: string, timeZone: string) =>
  invoke<T>("coach_insights", { day, timeZone });

export const updateMemory = <C>(command: C) => invoke<void>("coach_memory", { command });
