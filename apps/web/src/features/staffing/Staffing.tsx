import { useEffect, useState } from "react";
import * as api from "@/lib/api";
import type { StaffingSnapshot } from "@/lib/types";
import { StaffingDirectory } from "./StaffingDirectory";
import { StaffingCapabilityGrid, StaffingMetrics } from "./StaffingMetrics";
import { StaffingOperations } from "./StaffingOperations";

const empty: StaffingSnapshot = {
  freelancers: [],
  skills: [],
  availability: [],
  requests: [],
  assignments: [],
  contracts: [],
  pipeline: [],
  report: {
    activeFreelancers: 0,
    availableMinutes: 0,
    openRequests: 0,
    activeAssignments: 0,
    plannedMinutes: 0,
    forecastRevenueCents: 0,
    forecastCostCents: 0,
    forecastMarginCents: 0,
    pipelineByStage: [],
  },
};

export function Staffing() {
  const [data, setData] = useState<StaffingSnapshot>(empty);
  const [error, setError] = useState<string>();
  const reload = async () => {
    try {
      setData(await api.staffingSnapshot());
      setError(undefined);
    } catch (e) {
      setError(api.describeError(e));
    }
  };
  useEffect(() => {
    void reload();
  }, []);
  const { report } = data;
  return (
    <main className="min-h-0 flex-1 overflow-y-auto bg-canvas p-6">
      <div className="mx-auto max-w-7xl space-y-6">
        <header>
          <p className="text-xs font-semibold tracking-widest text-accent uppercase">Operations</p>
          <h1 className="mt-1 text-3xl font-semibold text-fg-strong">Staffing & Freelancers</h1>
          <p className="mt-2 max-w-3xl text-sm text-fg-muted">
            People, capacity, matching, assignments, contracts, pipeline and margin in one durable
            local workspace.
          </p>
        </header>
        {error && (
          <div
            role="alert"
            className="rounded-lg border border-danger/30 bg-danger-soft p-3 text-sm text-danger"
          >
            {error}
          </div>
        )}
        <StaffingMetrics report={report} />
        <StaffingDirectory freelancers={data.freelancers} onSaved={reload} onError={setError} />
        <StaffingOperations data={data} onSaved={reload} onError={setError} />
        <StaffingCapabilityGrid
          data={{
            availability: data.availability.length,
            requests: data.requests.length,
            assignments: data.assignments.length,
            contracts: data.contracts.length,
            pipeline: data.pipeline.length,
            skills: data.skills.length,
            report,
          }}
        />
      </div>
    </main>
  );
}
