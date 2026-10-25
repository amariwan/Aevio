import { useEffect, useState } from "react";
import { Toggle } from "@/components/primitives/SegmentedControl";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useI18n } from "@/hooks/useI18n";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import * as api from "@/lib/api";
import type { DecisionHealth, DecisionProviderMode, DecisionSettings } from "@/lib/types";

const DEFAULTS: DecisionSettings = {
  enabled: true,
  mode: "auto",
  layaEndpoint: "http://127.0.0.1:8000/predict",
  layaCheckpoint: "typed-decisions",
  jevModel: "typesafe/jev-1.14",
  allowCloud: false,
  timeoutMs: 2000,
  minimumConfidencePercent: 80,
  humanReview: true,
};

const MODES: { value: DecisionProviderMode; label: string }[] = [
  { value: "auto", label: "Automatic (local first)" },
  { value: "local_only", label: "Local only" },
  { value: "rules_only", label: "Rules only" },
  { value: "cloud", label: "Cloud (explicit)" },
  { value: "disabled", label: "Disabled" },
];

export function DecisionModelsSettings() {
  const { t } = useI18n();
  const [decision, setDecision] = useState<DecisionSettings>(DEFAULTS);
  const [health, setHealth] = useState<DecisionHealth | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    void Promise.all([api.getDecisionSettings(), api.decisionHealth()])
      .then(([loaded, nextHealth]) => {
        setDecision(loaded);
        setHealth(nextHealth);
      })
      .catch((cause: unknown) => setError(api.describeError(cause)));
  }, []);

  const update = <K extends keyof DecisionSettings>(key: K, value: DecisionSettings[K]): void => {
    setDecision((current) => ({ ...current, [key]: value }));
  };

  const save = (): void => {
    setSaving(true);
    setError(null);
    void api
      .updateDecisionSettings(decision)
      .then(setDecision)
      .catch((cause: unknown) => setError(api.describeError(cause)))
      .finally(() => setSaving(false));
  };

  return (
    <div className="flex flex-col gap-4 rounded-xl border border-line bg-surface p-4">
      <div>
        <h3 className="text-[13px] font-medium text-fg">{t("decisionModels.typedDecisions")}</h3>
        <p className="mt-1 text-[12px] leading-relaxed text-fg-muted">
          Focus recovery, task prioritization, and daily review use bounded questions. Coach
          policies still own scheduling and intervention.
        </p>
      </div>
      <div className="grid gap-3 md:grid-cols-2">
        <label className="flex items-center justify-between gap-3 text-[12px] text-fg">
          <span>{t("decisionModels.enableEngine")}</span>
          <Toggle
            checked={decision.enabled}
            label="Enable decision engine"
            onChange={(value) => update("enabled", value)}
          />
        </label>
        <label className="flex items-center justify-between gap-3 text-[12px] text-fg">
          <span>{t("decisionModels.humanReview")}</span>
          <Toggle
            checked={decision.humanReview}
            label="Human review below threshold"
            onChange={(value) => update("humanReview", value)}
          />
        </label>
      </div>
      <div className="grid gap-3 md:grid-cols-2">
        <div className="grid gap-1.5">
          <Label htmlFor="decision-mode">{t("decisionModels.providerMode")}</Label>
          <NativeSelect
            id="decision-mode"
            value={decision.mode}
            onChange={(event) => {
              const selected = MODES.find((mode) => mode.value === event.target.value)?.value;
              if (selected) update("mode", selected);
            }}
          >
            {MODES.map((mode) => (
              <NativeSelectOption key={mode.value} value={mode.value}>
                {mode.label}
              </NativeSelectOption>
            ))}
          </NativeSelect>
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor="decision-confidence">Minimum confidence (%)</Label>
          <Input
            id="decision-confidence"
            type="number"
            min={1}
            max={100}
            value={decision.minimumConfidencePercent}
            onChange={(event) => update("minimumConfidencePercent", Number(event.target.value))}
          />
        </div>
      </div>
      <div className="grid gap-3 md:grid-cols-2">
        <div className="grid gap-1.5">
          <Label htmlFor="laya-endpoint">{t("decisionModels.layaEndpoint")}</Label>
          <Input
            id="laya-endpoint"
            value={decision.layaEndpoint}
            onChange={(event) => update("layaEndpoint", event.target.value)}
          />
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor="laya-checkpoint">{t("decisionModels.layaCheckpoint")}</Label>
          <Input
            id="laya-checkpoint"
            value={decision.layaCheckpoint}
            onChange={(event) => update("layaCheckpoint", event.target.value)}
          />
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor="jev-model">{t("decisionModels.jevModel")}</Label>
          <Input
            id="jev-model"
            value={decision.jevModel}
            onChange={(event) => update("jevModel", event.target.value)}
          />
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor="decision-timeout">Timeout (ms)</Label>
          <Input
            id="decision-timeout"
            type="number"
            min={100}
            max={30000}
            value={decision.timeoutMs}
            onChange={(event) => update("timeoutMs", Number(event.target.value))}
          />
        </div>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3 border-t border-line pt-3">
        <div className="text-[11px] text-fg-muted">
          Rules: {health?.rules ?? "loading"} · Laya: {health?.laya ?? "loading"} · Jev:{" "}
          {health?.jev ?? "loading"}
          {health?.cloudConfigured ? " · backend key configured" : ""}
        </div>
        <Button type="button" size="sm" onClick={save} disabled={saving}>
          {saving ? "Saving…" : "Save decision settings"}
        </Button>
      </div>
      {error && (
        <p role="alert" className="text-[12px] text-danger">
          {error}
        </p>
      )}
      <p className="text-[11px] leading-relaxed text-fg-muted">
        Remote inference is off by default. API keys are read only from the backend environment and
        are never sent to the webview.
      </p>
    </div>
  );
}
