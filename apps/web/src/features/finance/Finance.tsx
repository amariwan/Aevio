import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import SpendingRing from "@/components/kokonutui/spending-ring";
import { useI18n } from "@/hooks/useI18n";
import { Input } from "@/components/ui/input";
import { spendingCategories } from "@/features/finance/spending-categories";
import { useCatalog } from "@/hooks/useCatalog";
import * as api from "@/lib/api";
import type {
  FinanceCapacityPlan,
  FinanceContract,
  FinanceRateCard,
  FinanceSnapshot,
} from "@/lib/types";

const money = (cents: number): string =>
  new Intl.NumberFormat(undefined, { style: "currency", currency: "USD" }).format(cents / 100);
const hours = (ms: number): string => `${(ms / 3_600_000).toFixed(1)} h`;
const dateInput = (date: Date): string => date.toISOString().slice(0, 10);

export function Finance() {
  const { t } = useI18n();
  const catalog = useCatalog();
  const today = new Date();
  const [start, setStart] = useState(dateInput(new Date(today.getFullYear(), today.getMonth(), 1)));
  const [end, setEnd] = useState(dateInput(new Date(today.getFullYear(), today.getMonth() + 1, 0)));
  const [tab, setTab] = useState<"overview" | "planning" | "commercial">("overview");
  const [snapshot, setSnapshot] = useState<FinanceSnapshot | null>(null);
  const [plans, setPlans] = useState<FinanceCapacityPlan[]>([]);
  const [rates, setRates] = useState<FinanceRateCard[]>([]);
  const [contracts, setContracts] = useState<FinanceContract[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [shareUrl, setShareUrl] = useState<string | null>(null);
  const [planProject, setPlanProject] = useState("");
  const [planPerson, setPlanPerson] = useState("default");
  const [planMinutes, setPlanMinutes] = useState("1600");
  const [rateProject, setRateProject] = useState("");
  const [billRate, setBillRate] = useState("0");
  const [costRate, setCostRate] = useState("0");
  const [contractClient, setContractClient] = useState("");
  const [contractKind, setContractKind] = useState<FinanceContract["kind"]>("retainer");
  const [contractAmount, setContractAmount] = useState("0");
  const [contractHours, setContractHours] = useState("0");

  const load = async (): Promise<void> => {
    setBusy(true);
    setError(null);
    try {
      const from = new Date(`${start}T00:00:00`).getTime();
      const until = new Date(`${end}T23:59:59.999`).getTime();
      const [nextSnapshot, nextPlans, nextRates, nextContracts] = await Promise.all([
        api.financeSnapshot(from, until),
        api.listFinanceCapacityPlans(),
        api.listFinanceRateCards(),
        api.listFinanceContracts(),
      ]);
      setSnapshot(nextSnapshot);
      setPlans(nextPlans);
      setRates(nextRates);
      setContracts(nextContracts);
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  useEffect(() => {
    void load();
  }, []);

  const savePlan = async (): Promise<void> => {
    setBusy(true);
    try {
      await api.saveFinanceCapacityPlan({
        projectId: planProject || undefined,
        personKey: planPerson,
        periodStart: new Date(`${start}T00:00:00`).getTime(),
        periodEnd: new Date(`${end}T23:59:59.999`).getTime(),
        plannedMinutes: Number(planMinutes),
      });
      await load();
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  const saveRate = async (): Promise<void> => {
    if (!rateProject) return;
    setBusy(true);
    try {
      await api.saveFinanceRateCard({
        projectId: rateProject,
        billRateCents: Math.round(Number(billRate) * 100),
        costRateCents: Math.round(Number(costRate) * 100),
        currency: "USD",
      });
      await load();
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  const saveContract = async (): Promise<void> => {
    if (!contractClient) return;
    setBusy(true);
    try {
      await api.saveFinanceContract({
        clientId: contractClient,
        kind: contractKind,
        currency: "USD",
        startDate: start,
        endDate: end,
        recurringCents: Math.round(Number(contractAmount) * 100),
        includedHoursHundredths: Math.round(Number(contractHours) * 100),
        overageRateCents: Math.round(Number(billRate) * 100),
        status: "active",
      });
      await load();
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };
  const share = async (): Promise<void> => {
    setBusy(true);
    try {
      const dashboard = await api.createDashboard({
        name: `Finance ${start} – ${end}`,
        filter: { start, end },
      });
      const result = await api.shareFinanceDashboard(dashboard.id);
      setShareUrl(`aevio://dashboard/${result.token}`);
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setBusy(false);
    }
  };

  const spendingByProject = snapshot ? spendingCategories(snapshot.rows) : [];

  return (
    <main className="min-h-0 flex-1 overflow-y-auto bg-canvas px-6 py-7">
      <div className="mx-auto flex max-w-6xl flex-col gap-6">
        <header className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.18em] text-fg-muted">
              {t("finance.analyzeAndFinance")}
            </p>
            <h1 className="mt-1 text-2xl font-semibold text-fg-strong">
              {t("finance.profitabilityCockpit")}
            </h1>
            <p className="mt-1 text-sm text-fg-muted">{t("finance.subtitle")}</p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Input
              type="date"
              value={start}
              onChange={(event) => setStart(event.target.value)}
              className="w-36"
            />
            <span className="text-fg-muted">–</span>
            <Input
              type="date"
              value={end}
              onChange={(event) => setEnd(event.target.value)}
              className="w-36"
            />
            <Button type="button" onClick={() => void load()} disabled={busy}>
              {busy ? t("finance.loading") : t("finance.refresh")}
            </Button>
            <Button type="button" variant="outline" onClick={() => void share()} disabled={busy}>
              {t("finance.shareDashboard")}
            </Button>
          </div>
        </header>
        {error && (
          <p role="alert" className="rounded-md bg-danger-soft px-3 py-2 text-sm text-danger">
            {error}
          </p>
        )}
        {shareUrl && (
          <p className="rounded-md bg-success-soft px-3 py-2 text-sm text-success">
            {t("finance.shareCreated", { url: shareUrl })}
          </p>
        )}
        <div className="flex gap-1 border-b border-line">
          {(["overview", "planning", "commercial"] as const).map((item) => (
            <Button
              key={item}
              type="button"
              variant={tab === item ? "secondary" : "ghost"}
              onClick={() => setTab(item)}
            >
              {item === "overview"
                ? t("finance.overviewTab")
                : item === "planning"
                  ? t("finance.planningTab")
                  : t("finance.commercialTab")}
            </Button>
          ))}
        </div>
        {tab === "overview" && snapshot && (
          <>
            <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
              {[
                [t("finance.tracked"), hours(snapshot.trackedMs)],
                [t("finance.planned"), hours(snapshot.plannedMs)],
                [t("finance.billable"), hours(snapshot.billableMs)],
                [t("finance.revenue"), money(snapshot.revenueCents)],
                [
                  t("finance.margin"),
                  `${money(snapshot.marginCents)}${snapshot.marginPercent == null ? "" : ` · ${snapshot.marginPercent.toFixed(1)}%`}`,
                ],
              ].map(([label, value]) => (
                <Card key={label}>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-xs font-medium text-fg-muted">{label}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <p className="text-xl font-semibold text-fg-strong">{value}</p>
                  </CardContent>
                </Card>
              ))}
            </section>
            <SpendingRing
              title={t("finance.costsByProject")}
              categories={spendingByProject}
              currency="USD"
              className="max-w-none"
            />
            <Card>
              <CardHeader>
                <CardTitle>{t("finance.trackedTimePivot")}</CardTitle>
              </CardHeader>
              <CardContent className="overflow-x-auto p-0">
                <table className="w-full min-w-[820px] text-sm">
                  <thead className="border-b border-line text-left text-xs text-fg-muted">
                    <tr>
                      <th className="px-5 py-3">{t("finance.projectClient")}</th>
                      <th className="px-3 py-3 text-right">{t("finance.tracked")}</th>
                      <th className="px-3 py-3 text-right">{t("finance.planned")}</th>
                      <th className="px-3 py-3 text-right">{t("finance.billable")}</th>
                      <th className="px-3 py-3 text-right">{t("finance.revenue")}</th>
                      <th className="px-3 py-3 text-right">{t("finance.cost")}</th>
                      <th className="px-5 py-3 text-right">{t("finance.margin")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {snapshot.rows.map((row) => (
                      <tr
                        key={row.projectId ?? "none"}
                        className="border-b border-line last:border-0"
                      >
                        <td className="px-5 py-3">
                          <div className="font-medium text-fg-strong">{row.projectName}</div>
                          <div className="text-xs text-fg-muted">
                            {row.clientName ?? t("finance.noClient")}
                          </div>
                        </td>
                        <td className="px-3 py-3 text-right">{hours(row.trackedMs)}</td>
                        <td className="px-3 py-3 text-right">{hours(row.plannedMs)}</td>
                        <td className="px-3 py-3 text-right">{hours(row.billableMs)}</td>
                        <td className="px-3 py-3 text-right">{money(row.revenueCents)}</td>
                        <td className="px-3 py-3 text-right">{money(row.costCents)}</td>
                        <td
                          className={`px-5 py-3 text-right font-medium ${row.marginCents < 0 ? "text-danger" : "text-success"}`}
                        >
                          {money(row.marginCents)}
                          {row.marginPercent == null ? "" : ` · ${row.marginPercent.toFixed(1)}%`}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {snapshot.rows.length === 0 && (
                  <p className="px-5 py-8 text-center text-sm text-fg-muted">
                    {t("finance.noTimeEntries")}
                  </p>
                )}
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>{t("finance.marginBudgetWarnings")}</CardTitle>
              </CardHeader>
              <CardContent>
                {snapshot.warnings.length === 0 ? (
                  <p className="text-sm text-success">{t("finance.noWarnings")}</p>
                ) : (
                  <div className="flex flex-col gap-2">
                    {snapshot.warnings.map((warning, index) => (
                      <div
                        key={`${warning.kind}-${warning.projectId ?? "none"}-${index}`}
                        className="rounded-md bg-warning-soft px-3 py-2 text-sm text-fg"
                      >
                        <span className="font-semibold">{warning.severity}:</span> {warning.message}
                      </div>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          </>
        )}
        {tab === "planning" && (
          <section className="grid gap-4 lg:grid-cols-[1fr_1.2fr]">
            <Card>
              <CardHeader>
                <CardTitle>{t("finance.planCapacity")}</CardTitle>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                <label className="text-sm text-fg-muted">
                  {t("finance.project")}
                  <select
                    value={planProject}
                    onChange={(event) => setPlanProject(event.target.value)}
                    className="mt-1 h-8 w-full rounded-md border border-line bg-surface px-2 text-sm text-fg"
                  >
                    <option value="">{t("finance.allOverhead")}</option>
                    {catalog.projects.map((project) => (
                      <option key={project.id} value={project.id}>
                        {project.name}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="text-sm text-fg-muted">
                  {t("finance.personTeam")}
                  <input
                    value={planPerson}
                    onChange={(event) => setPlanPerson(event.target.value)}
                    className="mt-1 h-8 w-full rounded-md border border-line bg-surface px-2 text-sm text-fg"
                  />
                </label>
                <label className="text-sm text-fg-muted">
                  {t("finance.plannedMinutes")}
                  <input
                    type="number"
                    min="0"
                    value={planMinutes}
                    onChange={(event) => setPlanMinutes(event.target.value)}
                    className="mt-1 h-8 w-full rounded-md border border-line bg-surface px-2 text-sm text-fg"
                  />
                </label>
                <Button type="button" onClick={() => void savePlan()} disabled={busy}>
                  {t("finance.saveCapacityPlan")}
                </Button>
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>{t("finance.currentPlanning")}</CardTitle>
              </CardHeader>
              <CardContent className="flex flex-col gap-2">
                {plans.length === 0 ? (
                  <p className="text-sm text-fg-muted">{t("finance.noCapacityPlans")}</p>
                ) : (
                  plans.map((plan) => (
                    <div
                      key={plan.id}
                      className="flex justify-between rounded-md border border-line px-3 py-2 text-sm"
                    >
                      <span>
                        {plan.personKey} ·{" "}
                        {catalog.projectById.get(plan.projectId ?? "")?.name ??
                          t("finance.overhead")}
                      </span>
                      <span className="text-fg-muted">
                        {(plan.plannedMinutes / 60).toFixed(1)} h
                      </span>
                    </div>
                  ))
                )}
              </CardContent>
            </Card>
          </section>
        )}
        {tab === "commercial" && (
          <section className="grid gap-4 xl:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle>{t("finance.billRatesCostRates")}</CardTitle>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                <select
                  value={rateProject}
                  onChange={(event) => setRateProject(event.target.value)}
                  className="h-8 rounded-md border border-line bg-surface px-2 text-sm text-fg"
                >
                  <option value="">{t("finance.chooseProject")}</option>
                  {catalog.projects.map((project) => (
                    <option key={project.id} value={project.id}>
                      {project.name}
                    </option>
                  ))}
                </select>
                <div className="grid grid-cols-2 gap-2">
                  <Input
                    type="number"
                    min="0"
                    value={billRate}
                    onChange={(event) => setBillRate(event.target.value)}
                    placeholder={t("finance.billRate")}
                  />
                  <Input
                    type="number"
                    min="0"
                    value={costRate}
                    onChange={(event) => setCostRate(event.target.value)}
                    placeholder={t("finance.costRate")}
                  />
                </div>
                <Button
                  type="button"
                  onClick={() => void saveRate()}
                  disabled={busy || !rateProject}
                >
                  {t("finance.saveRateCard")}
                </Button>
                {rates.map((rate) => (
                  <div key={rate.projectId} className="flex justify-between text-sm text-fg-muted">
                    <span>{catalog.projectById.get(rate.projectId)?.name ?? rate.projectId}</span>
                    <span>
                      {money(rate.billRateCents)} / {money(rate.costRateCents)}
                    </span>
                  </div>
                ))}
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>{t("finance.retainerHybridContracts")}</CardTitle>
              </CardHeader>
              <CardContent className="flex flex-col gap-3">
                <select
                  value={contractClient}
                  onChange={(event) => setContractClient(event.target.value)}
                  className="h-8 rounded-md border border-line bg-surface px-2 text-sm text-fg"
                >
                  <option value="">{t("finance.chooseClient")}</option>
                  {catalog.clients.map((client) => (
                    <option key={client.id} value={client.id}>
                      {client.name}
                    </option>
                  ))}
                </select>
                <select
                  value={contractKind}
                  onChange={(event) =>
                    setContractKind(event.target.value as FinanceContract["kind"])
                  }
                  className="h-8 rounded-md border border-line bg-surface px-2 text-sm text-fg"
                >
                  <option value="fixed">{t("finance.fixed")}</option>
                  <option value="retainer">{t("finance.retainer")}</option>
                  <option value="hybrid">{t("finance.hybrid")}</option>
                </select>
                <div className="grid grid-cols-2 gap-2">
                  <Input
                    type="number"
                    min="0"
                    value={contractAmount}
                    onChange={(event) => setContractAmount(event.target.value)}
                    placeholder={t("finance.recurring")}
                  />
                  <Input
                    type="number"
                    min="0"
                    value={contractHours}
                    onChange={(event) => setContractHours(event.target.value)}
                    placeholder={t("finance.inclusiveHours")}
                  />
                </div>
                <Button
                  type="button"
                  onClick={() => void saveContract()}
                  disabled={busy || !contractClient}
                >
                  {t("finance.saveContract")}
                </Button>
                {contracts.map((contract) => (
                  <div key={contract.id} className="flex justify-between text-sm text-fg-muted">
                    <span>
                      {catalog.clientById.get(contract.clientId)?.name ?? contract.clientId} ·{" "}
                      {contract.kind}
                    </span>
                    <span>{money(contract.recurringCents)}</span>
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
