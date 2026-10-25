import type { StaffingReport } from "@/lib/types";

export function StaffingMetrics({ report }: { report: StaffingReport }) {
  const metrics = [
    ["Active freelancers", report.activeFreelancers],
    ["Available hours", `${Math.round(report.availableMinutes / 60)}h`],
    ["Open requests", report.openRequests],
    ["Forecast margin", `${(report.forecastMarginCents / 100).toFixed(2)} USD`],
  ];
  return (
    <section className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      {metrics.map(([label, value]) => (
        <div key={label} className="rounded-xl border border-line bg-surface p-4">
          <div className="text-xs text-fg-muted">{label}</div>
          <div className="mt-1 text-2xl font-semibold text-fg-strong">{value}</div>
        </div>
      ))}
    </section>
  );
}

export function StaffingCapabilityGrid({
  data,
}: {
  data: {
    availability: number;
    requests: number;
    assignments: number;
    contracts: number;
    pipeline: number;
    skills: number;
    report: StaffingReport;
  };
}) {
  const { report } = data;
  const cards = [
    [
      "Availability & capacity",
      `${data.availability} availability windows`,
      `${Math.round(report.availableMinutes / 60)}h available`,
    ],
    [
      "Staffing & matching",
      `${data.requests} requests`,
      `${report.openRequests} open for matching`,
    ],
    [
      "Assignments",
      `${data.assignments} assignments`,
      `${Math.round(report.plannedMinutes / 60)}h planned`,
    ],
    ["Contracts & documents", `${data.contracts} contracts`, "Expiry and notice tracking"],
    [
      "Freelancer pipeline",
      `${data.pipeline} candidates`,
      `${report.pipelineByStage.length} active stages`,
    ],
    [
      "Financial planning",
      `${(report.forecastRevenueCents / 100).toFixed(2)} USD revenue`,
      `${(report.forecastCostCents / 100).toFixed(2)} USD cost`,
    ],
    [
      "Reporting",
      `${report.activeAssignments} active assignments`,
      `${(report.forecastMarginCents / 100).toFixed(2)} USD margin`,
    ],
    ["Skills & profiles", `${data.skills} skills`, "Matching-ready catalog"],
  ];
  return (
    <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      {cards.map(([title, primary, secondary]) => (
        <article key={title} className="rounded-xl border border-line bg-surface p-4">
          <h2 className="font-medium text-fg-strong">{title}</h2>
          <p className="mt-2 text-sm text-fg-soft">{primary}</p>
          <p className="mt-1 text-xs text-fg-muted">{secondary}</p>
        </article>
      ))}
    </section>
  );
}
