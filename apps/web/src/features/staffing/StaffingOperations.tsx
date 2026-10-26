import { useState, type ReactNode } from "react";
import { useI18n } from "@/hooks/useI18n";
import * as api from "@/lib/api";
import type { StaffingMatch, StaffingSnapshot } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";

type Fields = Record<string, string>;

const DIALOGS = ["availability", "request", "assignment", "contract", "pipeline", "skill"] as const;
type DialogKey = (typeof DIALOGS)[number];

export function StaffingOperations({
  data,
  onSaved,
  onError,
}: {
  data: StaffingSnapshot;
  onSaved: () => Promise<void>;
  onError: (message: string) => void;
}) {
  const { t } = useI18n();
  const TITLES: Record<DialogKey, string> = {
    availability: t("staffing.availabilityWindow"),
    request: t("staffing.staffingRequest"),
    assignment: t("staffing.assignment"),
    contract: t("staffing.freelancerContract"),
    pipeline: t("staffing.pipelineEntry"),
    skill: t("staffing.skillCatalog"),
  };
  const [fields, setFields] = useState<Fields>({});
  const [active, setActive] = useState<DialogKey | null>(null);
  const [selectedRequest, setSelectedRequest] = useState("");
  const [matches, setMatches] = useState<StaffingMatch[]>([]);

  const update = (name: string, value: string) =>
    setFields((current) => ({ ...current, [name]: value }));

  const save = async (kind: string) => {
    try {
      const payload: Record<string, unknown> = { ...fields };
      if (kind === "request")
        payload.skills = (fields.skills || "")
          .split(",")
          .map((skill) => skill.trim())
          .filter(Boolean);
      await api.saveStaffingRecord(kind, payload);
      setFields({});
      setActive(null);
      await onSaved();
    } catch (error) {
      onError(api.describeError(error));
    }
  };

  const match = async () => {
    if (!selectedRequest) return;
    try {
      setMatches(await api.staffingMatches(selectedRequest));
    } catch (error) {
      onError(api.describeError(error));
    }
  };

  const freelancerSelect = (
    <NativeSelect
      aria-label={t("staffing.freelancer")}
      value={fields.freelancerId || ""}
      onChange={(event) => update("freelancerId", event.target.value)}
    >
      <NativeSelectOption value="">{t("staffing.freelancer")}</NativeSelectOption>
      {data.freelancers.map((person) => (
        <NativeSelectOption key={person.id} value={person.id}>
          {person.name}
        </NativeSelectOption>
      ))}
    </NativeSelect>
  );

  const field = (name: string, placeholder: string) => (
    <Input
      value={fields[name] || ""}
      onChange={(event) => update(name, event.target.value)}
      placeholder={placeholder}
      aria-label={placeholder}
    />
  );

  const renderForm = (key: DialogKey): ReactNode => {
    switch (key) {
      case "availability":
        return (
          <>
            {freelancerSelect}
            {field("startDate", t("staffing.startDate"))}
            {field("endDate", t("staffing.endDate"))}
            {field("availableMinutes", t("staffing.availableMinutes"))}
          </>
        );
      case "request":
        return (
          <>
            {field("role", t("staffing.role"))}
            {field("skills", t("staffing.requiredSkills"))}
            {field("startDate", t("staffing.startDate"))}
            {field("endDate", t("staffing.endDate"))}
            {field("requestedMinutes", t("staffing.requestedMinutes"))}
          </>
        );
      case "assignment":
        return (
          <>
            {freelancerSelect}
            {field("role", t("staffing.assignmentRole"))}
            {field("startDate", t("staffing.startDate"))}
            {field("endDate", t("staffing.endDate"))}
            {field("plannedMinutes", t("staffing.plannedMinutes"))}
            {field("buyRateCents", t("staffing.buyRateCents"))}
            {field("sellRateCents", t("staffing.sellRateCents"))}
          </>
        );
      case "contract":
        return (
          <>
            {freelancerSelect}
            {field("kind", t("staffing.contractType"))}
            {field("startDate", t("staffing.startDate"))}
            {field("endDate", t("staffing.endDateOptional"))}
            {field("noticeDays", t("staffing.noticeDays"))}
            {field("documentRef", t("staffing.documentReference"))}
          </>
        );
      case "pipeline":
        return (
          <>
            {freelancerSelect}
            <NativeSelect
              aria-label={t("staffing.pipelineStage")}
              value={fields.stage || "lead"}
              onChange={(event) => update("stage", event.target.value)}
            >
              <NativeSelectOption value="lead">{t("staffing.lead")}</NativeSelectOption>
              <NativeSelectOption value="screening">{t("staffing.screening")}</NativeSelectOption>
              <NativeSelectOption value="interview">{t("staffing.interview")}</NativeSelectOption>
              <NativeSelectOption value="offer">{t("staffing.offer")}</NativeSelectOption>
              <NativeSelectOption value="onboarding">{t("staffing.onboarding")}</NativeSelectOption>
              <NativeSelectOption value="active">{t("staffing.active")}</NativeSelectOption>
            </NativeSelect>
            {field("notes", t("staffing.notes"))}
          </>
        );
      case "skill":
        return (
          <>
            {field("name", t("staffing.skillName"))}
            {field("category", t("staffing.category"))}
          </>
        );
    }
  };

  return (
    <section className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold text-fg-strong">{t("staffing.operations")}</h2>
        <p className="text-sm text-fg-muted">{t("staffing.operationsDescription")}</p>
      </div>

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {DIALOGS.map((key) => (
          <Card key={key} className="border-line/70 bg-surface">
            <CardHeader className="pb-2">
              <CardTitle className="text-base text-fg-strong">{TITLES[key]}</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="mb-3 text-xs text-fg-muted">
                {t("staffing.createRecord", { title: TITLES[key] })}
              </p>
              <Button size="sm" onClick={() => setActive(key)}>
                {t("staffing.openForm")}
              </Button>
            </CardContent>
          </Card>
        ))}
      </div>

      <Dialog open={active !== null} onOpenChange={(o) => !o && setActive(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{active ? TITLES[active] : ""}</DialogTitle>
            <DialogDescription>{t("staffing.fillDetails")}</DialogDescription>
          </DialogHeader>
          <div className="space-y-2">{active && renderForm(active)}</div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setActive(null)}>
              {t("staffing.cancel")}
            </Button>
            <Button onClick={() => active && void save(active)} disabled={!active}>
              {t("staffing.save")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Card className="border-line/70 bg-surface">
        <CardHeader className="pb-2">
          <CardTitle className="text-base text-fg-strong">{t("staffing.matching")}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex flex-wrap items-center gap-3">
            <NativeSelect
              aria-label={t("staffing.requestToMatch")}
              value={selectedRequest}
              onChange={(event) => setSelectedRequest(event.target.value)}
            >
              <NativeSelectOption value="">{t("staffing.selectRequest")}</NativeSelectOption>
              {data.requests.map((request) => (
                <NativeSelectOption key={request.id} value={request.id}>
                  {request.role} · {request.startDate}
                </NativeSelectOption>
              ))}
            </NativeSelect>
            <Button variant="outline" size="sm" onClick={() => void match()}>
              {t("staffing.findMatches")}
            </Button>
          </div>
          {matches.length > 0 && (
            <div className="grid gap-2 sm:grid-cols-2">
              {matches.map((candidate) => (
                <Card key={candidate.freelancerId} className="border-line/70">
                  <CardContent className="p-3 text-sm">
                    <div className="font-medium text-fg-strong">
                      {data.freelancers.find((person) => person.id === candidate.freelancerId)
                        ?.name || candidate.freelancerId}
                    </div>
                    <div className="text-xs text-fg-muted">
                      {t("staffing.skillMatch", {
                        score: Math.round(candidate.score * 100),
                        hours: Math.round(candidate.availableMinutes / 60),
                      })}
                    </div>
                    <div className="mt-1 text-xs text-fg-muted">
                      {candidate.matchedSkills.join(", ") || t("staffing.noMatchingSkills")}
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </section>
  );
}
