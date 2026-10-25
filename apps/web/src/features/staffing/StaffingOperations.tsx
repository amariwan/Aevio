import { useState, type ReactNode } from "react";
import * as api from "@/lib/api";
import type { StaffingMatch, StaffingSnapshot } from "@/lib/types";

type Fields = Record<string, string>;

function Input({
  fields,
  setFields,
  name,
  placeholder,
}: {
  fields: Fields;
  setFields: (fields: Fields) => void;
  name: string;
  placeholder: string;
}) {
  return (
    <input
      value={fields[name] || ""}
      onChange={(event) => setFields({ ...fields, [name]: event.target.value })}
      placeholder={placeholder}
      aria-label={placeholder}
      className="w-full rounded-lg border border-line bg-canvas px-3 py-2 text-sm"
    />
  );
}

export function StaffingOperations({
  data,
  onSaved,
  onError,
}: {
  data: StaffingSnapshot;
  onSaved: () => Promise<void>;
  onError: (message: string) => void;
}) {
  const [fields, setFields] = useState<Fields>({});
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
  const card = (title: string, kind: string, content: ReactNode) => (
    <form
      className="rounded-xl border border-line bg-surface p-4"
      onSubmit={(event) => {
        event.preventDefault();
        void save(kind);
      }}
    >
      <h3 className="font-medium text-fg-strong">{title}</h3>
      <div className="mt-3 space-y-2">{content}</div>
      <button
        type="submit"
        className="mt-3 rounded-lg bg-accent px-3 py-2 text-xs font-medium text-accent-foreground"
      >
        Save
      </button>
    </form>
  );
  const freelancerSelect = (
    <select
      aria-label="Staffing freelancer"
      value={fields.freelancerId || ""}
      onChange={(event) => update("freelancerId", event.target.value)}
      className="w-full rounded-lg border border-line bg-canvas px-3 py-2 text-sm"
    >
      <option value="">Freelancer</option>
      {data.freelancers.map((person) => (
        <option key={person.id} value={person.id}>
          {person.name}
        </option>
      ))}
    </select>
  );
  return (
    <section className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold text-fg-strong">Staffing operations</h2>
        <p className="text-sm text-fg-muted">
          Maintain capacity, demand, assignments, contracts and the freelancer pipeline.
        </p>
      </div>
      <div className="grid gap-4 lg:grid-cols-3">
        {card(
          "Availability window",
          "availability",
          <>
            {freelancerSelect}
            <Input
              fields={fields}
              setFields={setFields}
              name="startDate"
              placeholder="Start date (YYYY-MM-DD)"
            />
            <Input
              fields={fields}
              setFields={setFields}
              name="endDate"
              placeholder="End date (YYYY-MM-DD)"
            />
            <Input
              fields={fields}
              setFields={setFields}
              name="availableMinutes"
              placeholder="Available minutes"
            />
          </>,
        )}
        {card(
          "Staffing request",
          "request",
          <>
            <Input fields={fields} setFields={setFields} name="role" placeholder="Role" />
            <Input
              fields={fields}
              setFields={setFields}
              name="skills"
              placeholder="Required skills, comma separated"
            />
            <Input
              fields={fields}
              setFields={setFields}
              name="startDate"
              placeholder="Start date (YYYY-MM-DD)"
            />
            <Input
              fields={fields}
              setFields={setFields}
              name="endDate"
              placeholder="End date (YYYY-MM-DD)"
            />
            <Input
              fields={fields}
              setFields={setFields}
              name="requestedMinutes"
              placeholder="Requested minutes"
            />
          </>,
        )}
        {card(
          "Assignment",
          "assignment",
          <>
            {freelancerSelect}
            <Input
              fields={fields}
              setFields={setFields}
              name="role"
              placeholder="Assignment role"
            />
            <Input
              fields={fields}
              setFields={setFields}
              name="startDate"
              placeholder="Start date (YYYY-MM-DD)"
            />
            <Input
              fields={fields}
              setFields={setFields}
              name="endDate"
              placeholder="End date (YYYY-MM-DD)"
            />
            <Input
              fields={fields}
              setFields={setFields}
              name="plannedMinutes"
              placeholder="Planned minutes"
            />
            <Input
              fields={fields}
              setFields={setFields}
              name="buyRateCents"
              placeholder="Buy rate cents/hour"
            />
            <Input
              fields={fields}
              setFields={setFields}
              name="sellRateCents"
              placeholder="Sell rate cents/hour"
            />
          </>,
        )}
        {card(
          "Freelancer contract",
          "contract",
          <>
            {freelancerSelect}
            <Input fields={fields} setFields={setFields} name="kind" placeholder="Contract type" />
            <Input
              fields={fields}
              setFields={setFields}
              name="startDate"
              placeholder="Start date (YYYY-MM-DD)"
            />
            <Input
              fields={fields}
              setFields={setFields}
              name="endDate"
              placeholder="End date (optional)"
            />
            <Input
              fields={fields}
              setFields={setFields}
              name="noticeDays"
              placeholder="Notice days"
            />
            <Input
              fields={fields}
              setFields={setFields}
              name="documentRef"
              placeholder="Document reference"
            />
          </>,
        )}
        {card(
          "Pipeline entry",
          "pipeline",
          <>
            {freelancerSelect}
            <select
              aria-label="Pipeline stage"
              value={fields.stage || "lead"}
              onChange={(event) => update("stage", event.target.value)}
              className="w-full rounded-lg border border-line bg-canvas px-3 py-2 text-sm"
            >
              <option value="lead">Lead</option>
              <option value="screening">Screening</option>
              <option value="interview">Interview</option>
              <option value="offer">Offer</option>
              <option value="onboarding">Onboarding</option>
              <option value="active">Active</option>
            </select>
            <Input fields={fields} setFields={setFields} name="notes" placeholder="Notes" />
          </>,
        )}
        {card(
          "Skill catalog",
          "skill",
          <>
            <Input fields={fields} setFields={setFields} name="name" placeholder="Skill name" />
            <Input fields={fields} setFields={setFields} name="category" placeholder="Category" />
          </>,
        )}
      </div>
      <div className="rounded-xl border border-line bg-surface p-4">
        <div className="flex flex-wrap items-center gap-3">
          <h3 className="font-medium text-fg-strong">Matching</h3>
          <select
            aria-label="Staffing request to match"
            value={selectedRequest}
            onChange={(event) => setSelectedRequest(event.target.value)}
            className="rounded-lg border border-line bg-canvas px-3 py-2 text-sm"
          >
            <option value="">Select request</option>
            {data.requests.map((request) => (
              <option key={request.id} value={request.id}>
                {request.role} · {request.startDate}
              </option>
            ))}
          </select>
          <button
            type="button"
            onClick={() => void match()}
            className="rounded-lg border border-line px-3 py-2 text-xs font-medium"
          >
            Find matches
          </button>
        </div>
        {matches.length > 0 && (
          <div className="mt-3 grid gap-2 sm:grid-cols-2">
            {matches.map((candidate) => (
              <div
                key={candidate.freelancerId}
                className="rounded-lg border border-line p-3 text-sm"
              >
                <div className="font-medium text-fg-strong">
                  {data.freelancers.find((person) => person.id === candidate.freelancerId)?.name ||
                    candidate.freelancerId}
                </div>
                <div className="text-xs text-fg-muted">
                  {Math.round(candidate.score * 100)}% skill match ·{" "}
                  {Math.round(candidate.availableMinutes / 60)}h available
                </div>
                <div className="mt-1 text-xs text-fg-muted">
                  {candidate.matchedSkills.join(", ") || "No matching skills"}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
