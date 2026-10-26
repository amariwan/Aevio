import { useState } from "react";
import { useI18n } from "@/hooks/useI18n";
import * as api from "@/lib/api";
import type { StaffingFreelancer } from "@/lib/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";

export function StaffingDirectory({
  freelancers,
  onSaved,
  onError,
}: {
  freelancers: StaffingFreelancer[];
  onSaved: () => Promise<void>;
  onError: (message: string) => void;
}) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [status, setStatus] = useState("lead");
  const [busy, setBusy] = useState(false);

  const create = async () => {
    if (!name.trim()) return;
    setBusy(true);
    try {
      await api.saveStaffingFreelancer({
        name,
        email: email || undefined,
        status,
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      });
      setName("");
      setEmail("");
      setStatus("lead");
      setOpen(false);
      await onSaved();
    } catch (e) {
      onError(api.describeError(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold text-fg-strong">
            {t("staffing.freelancerDirectory")}
          </h2>
          <p className="text-xs text-fg-muted">
            {t("staffing.freelancerDirectoryDescription", { count: freelancers.length })}
          </p>
        </div>
        <Dialog open={open} onOpenChange={setOpen}>
          <DialogTrigger render={<Button />}>{t("staffing.addFreelancer")}</DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{t("staffing.addFreelancer")}</DialogTitle>
              <DialogDescription>{t("staffing.createFreelancerProfile")}</DialogDescription>
            </DialogHeader>
            <div className="space-y-3">
              <Input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={t("staffing.name")}
                aria-label={t("staffing.name")}
              />
              <Input
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder={t("staffing.email")}
                aria-label={t("staffing.email")}
                type="email"
              />
              <NativeSelect
                value={status}
                aria-label={t("staffing.freelancerStatus")}
                onChange={(e) => setStatus(e.target.value)}
              >
                <NativeSelectOption value="lead">{t("staffing.lead")}</NativeSelectOption>
                <NativeSelectOption value="active">{t("staffing.active")}</NativeSelectOption>
                <NativeSelectOption value="paused">{t("staffing.paused")}</NativeSelectOption>
                <NativeSelectOption value="archived">{t("staffing.archived")}</NativeSelectOption>
              </NativeSelect>
            </div>
            <DialogFooter>
              <Button type="button" disabled={busy || !name.trim()} onClick={() => void create()}>
                {busy ? t("staffing.saving") : t("staffing.saveFreelancer")}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {freelancers.map((person) => (
          <Card key={person.id} className="border-line/70 bg-surface">
            <CardHeader className="pb-2">
              <div className="flex items-center justify-between gap-2">
                <CardTitle className="text-base text-fg-strong">{person.name}</CardTitle>
                <Badge variant={person.status === "active" ? "default" : "secondary"}>
                  {person.status}
                </Badge>
              </div>
            </CardHeader>
            <CardContent className="space-y-1 text-xs text-fg-muted">
              <p>{person.email || t("staffing.noEmail")}</p>
              <p>{person.seniority || t("staffing.seniorityNotSet")}</p>
              <p>{person.location || t("staffing.locationNotSet")}</p>
              <p className="text-[10px] text-fg-faint">{person.timezone}</p>
            </CardContent>
          </Card>
        ))}
        {freelancers.length === 0 && (
          <p className="col-span-full py-8 text-sm text-fg-muted">
            {t("staffing.noFreelancerProfiles")}
          </p>
        )}
      </div>
    </section>
  );
}
