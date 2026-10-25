import { useEffect, useState } from "react";
import { useI18n } from "@/hooks/useI18n";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import * as api from "@/lib/api";
import type { GovernanceSnapshot } from "@/lib/types";

type Tab = "teams" | "members" | "departments" | "defaults" | "audit";

export function Teams({ tab = "teams" }: { tab?: Tab }) {
  const { t } = useI18n();
  const tabs: Array<[Tab, string]> = [
    ["teams", t("teams.teams")],
    ["members", t("teams.members")],
    ["departments", t("teams.departments")],
    ["defaults", t("teams.workspaceDefaults")],
    ["audit", t("teams.auditLog")],
  ];
  const [active, setActive] = useState<Tab>(tab);
  const [snapshot, setSnapshot] = useState<GovernanceSnapshot | null>(null);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [defaultsText, setDefaultsText] = useState("{}");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const load = async (): Promise<void> => {
    try {
      setSnapshot(await api.governanceSnapshot());
    } catch (cause) {
      setError(api.describeError(cause));
    }
  };
  useEffect(() => {
    void load();
  }, []);
  const addTeam = async (): Promise<void> => {
    if (!name.trim()) return;
    setBusy(true);
    try {
      await api.saveGovernanceTeam({
        name,
        currency: "USD",
        timezone: "UTC",
        rateStrategy: "member",
        autoApprove: false,
      });
      setName("");
      await load();
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  const addDepartment = async (): Promise<void> => {
    if (!name.trim()) return;
    setBusy(true);
    try {
      await api.saveGovernanceDepartment({ name });
      setName("");
      await load();
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  const addMember = async (): Promise<void> => {
    if (!email.trim() || !displayName.trim()) return;
    setBusy(true);
    try {
      await api.saveGovernanceMember({
        email,
        displayName,
        orgRole: "member",
        teamIds: snapshot?.teams[0] ? [snapshot.teams[0].id] : [],
        teamRole: "member",
        permissions: [],
        billableDefault: true,
      });
      setEmail("");
      setDisplayName("");
      await load();
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  const saveDefaults = async (): Promise<void> => {
    setBusy(true);
    try {
      await api.saveGovernanceDefaults(JSON.parse(defaultsText) as Record<string, unknown>, false);
      await load();
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  if (!snapshot)
    return <main className="flex-1 p-6 text-sm text-fg-muted">{error ?? "Lädt Governance…"}</main>;
  return (
    <main className="min-h-0 flex-1 overflow-y-auto bg-canvas px-6 py-7">
      <div className="mx-auto flex max-w-6xl flex-col gap-6">
        <header>
          <p className="text-xs font-semibold uppercase tracking-[0.18em] text-fg-muted">
            Workspace Administration
          </p>
          <h1 className="mt-1 text-2xl font-semibold text-fg-strong">Teams & Governance</h1>
          <p className="mt-1 text-sm text-fg-muted">
            Rollen, Departments, Defaults und jede administrative Änderung an einem Ort.
          </p>
          <p className="mt-2 text-xs text-fg-muted">
            Sicht:{" "}
            {snapshot.isViewer
              ? t("teams.viewer")
              : snapshot.isManager
                ? t("teams.manager")
                : t("teams.workspaceAdmin")}
            {snapshot.canManageResourcing ? " · Resourcing freigegeben" : ""}
            {snapshot.canViewFinance ? " · Finance sichtbar" : ""}
          </p>
        </header>
        <nav className="flex flex-wrap gap-2 border-b border-line pb-3">
          {tabs.map(([value, label]) => (
            <Button
              key={value}
              variant={active === value ? "default" : "outline"}
              size="sm"
              onClick={() => setActive(value)}
            >
              {label}
            </Button>
          ))}
        </nav>
        {error && (
          <p role="alert" className="rounded-md bg-danger-soft px-3 py-2 text-sm text-danger">
            {error}
          </p>
        )}
        {active === "teams" && (
          <Card>
            <CardHeader>
              <CardTitle>{t("teams.teams")}</CardTitle>
            </CardHeader>
            <CardContent className="flex flex-col gap-3">
              <div className="flex gap-2">
                <Input
                  placeholder={t("teams.teamName")}
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
                <Button disabled={busy || !snapshot.canManage} onClick={() => void addTeam()}>
                  Team anlegen
                </Button>
              </div>
              {snapshot.teams.map((team) => (
                <div
                  key={team.id}
                  className="flex items-center justify-between rounded-md border border-line px-3 py-3"
                >
                  <div>
                    <p className="font-medium text-fg-strong">{team.name}</p>
                    <p className="text-xs text-fg-muted">
                      {team.currency} · {team.timezone} · {team.memberCount} Members
                    </p>
                  </div>
                  <span className="text-xs text-fg-muted">
                    {team.autoApprove ? "Auto-Approve" : t("teams.reviewRequired")}
                  </span>
                </div>
              ))}
            </CardContent>
          </Card>
        )}
        {active === "members" && (
          <Card>
            <CardHeader>
              <CardTitle>Members & Permissions</CardTitle>
            </CardHeader>
            <CardContent className="flex flex-col gap-3 overflow-x-auto p-0">
              <div className="flex flex-wrap gap-2 px-5 pt-5">
                <Input
                  placeholder={t("teams.name")}
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                />
                <Input
                  placeholder={t("teams.email")}
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                />
                <Button disabled={busy || !snapshot.canManage} onClick={() => void addMember()}>
                  Member einladen
                </Button>
              </div>
              <table className="w-full text-sm">
                <thead className="border-b border-line text-left text-xs text-fg-muted">
                  <tr>
                    <th className="px-5 py-3">{t("teams.member")}</th>
                    <th className="px-3 py-3">{t("teams.orgRole")}</th>
                    <th className="px-3 py-3">{t("teams.teamRoles")}</th>
                    <th className="px-3 py-3">{t("teams.extraRights")}</th>
                    <th className="px-5 py-3">{t("teams.status")}</th>
                  </tr>
                </thead>
                <tbody>
                  {snapshot.members.map((member) => (
                    <tr key={member.id} className="border-b border-line last:border-0">
                      <td className="px-5 py-3">
                        <p className="font-medium text-fg-strong">{member.displayName}</p>
                        <p className="text-xs text-fg-muted">{member.email}</p>
                      </td>
                      <td className="px-3 py-3">{member.orgRole}</td>
                      <td className="px-3 py-3">{member.teamRoles.join(", ") || "–"}</td>
                      <td className="px-3 py-3">{member.permissions.join(", ") || "–"}</td>
                      <td className="px-5 py-3">{member.status}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </CardContent>
          </Card>
        )}
        {active === "departments" && (
          <Card>
            <CardHeader>
              <CardTitle>{t("teams.departments")}</CardTitle>
            </CardHeader>
            <CardContent className="flex flex-col gap-2">
              <div className="flex gap-2">
                <Input
                  placeholder={t("teams.departmentName")}
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
                <Button disabled={busy || !snapshot.canManage} onClick={() => void addDepartment()}>
                  Department anlegen
                </Button>
              </div>
              {snapshot.departments.map((department) => (
                <div
                  key={department.id}
                  className="flex justify-between rounded-md border border-line px-3 py-3"
                >
                  <span>{department.name}</span>
                  <span className="text-fg-muted">{department.memberCount} Members</span>
                </div>
              ))}
              {snapshot.departments.length === 0 && (
                <p className="text-sm text-fg-muted">Noch keine Departments angelegt.</p>
              )}
            </CardContent>
          </Card>
        )}
        {active === "defaults" && (
          <Card>
            <CardHeader>
              <CardTitle>{t("teams.workspaceDefaults")}</CardTitle>
            </CardHeader>
            <CardContent>
              <textarea
                className="min-h-40 w-full rounded-md border border-line bg-surface p-4 font-mono text-xs text-fg-muted"
                value={defaultsText}
                onChange={(e) => setDefaultsText(e.target.value)}
                aria-label={t("teams.workspaceDefaultsJson")}
              />
              <div className="mt-3 flex items-center justify-between">
                <p className="text-sm text-fg-muted">
                  Neue Members erben diese Vorgaben; die Änderung wird auditierbar gespeichert.
                </p>
                <Button disabled={busy || !snapshot.canManage} onClick={() => void saveDefaults()}>
                  Defaults speichern
                </Button>
              </div>
            </CardContent>
          </Card>
        )}
        {active === "audit" && (
          <Card>
            <CardHeader>
              <CardTitle>{t("teams.auditLog")}</CardTitle>
            </CardHeader>
            <CardContent className="flex flex-col gap-2">
              {snapshot.audit.map((event) => (
                <div key={event.id} className="rounded-md border border-line px-3 py-2 text-sm">
                  <span className="font-medium text-fg-strong">{event.action}</span>{" "}
                  <span className="text-fg-muted">
                    {event.entityType}/{event.entityId} · {new Date(event.at).toLocaleString()}
                  </span>
                </div>
              ))}
            </CardContent>
          </Card>
        )}
      </div>
    </main>
  );
}
