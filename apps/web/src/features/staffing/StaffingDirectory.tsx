import { useState } from "react";
import * as api from "@/lib/api";
import type { StaffingFreelancer } from "@/lib/types";

export function StaffingDirectory({
  freelancers,
  onSaved,
  onError,
}: {
  freelancers: StaffingFreelancer[];
  onSaved: () => Promise<void>;
  onError: (message: string) => void;
}) {
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
      await onSaved();
    } catch (e) {
      onError(api.describeError(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="grid gap-6 lg:grid-cols-[1.2fr_1fr]">
      <div className="rounded-xl border border-line bg-surface p-5">
        <div className="mb-4 flex items-center justify-between">
          <div>
            <h2 className="font-semibold text-fg-strong">Freelancer directory</h2>
            <p className="text-xs text-fg-muted">
              Profiles, seniority, skills and engagement status.
            </p>
          </div>
          <span className="text-xs text-fg-muted">{freelancers.length} profiles</span>
        </div>
        <div className="grid gap-2 sm:grid-cols-2">
          {freelancers.map((person) => (
            <div key={person.id} className="rounded-lg border border-line p-3">
              <div className="font-medium text-fg-strong">{person.name}</div>
              <div className="text-xs text-fg-muted">
                {person.email || "No email"} · {person.status}
              </div>
              <div className="mt-2 text-xs text-fg-muted">
                {person.seniority || "Seniority not set"} · {person.location || "Location not set"}
              </div>
            </div>
          ))}
        </div>
      </div>
      <div className="rounded-xl border border-line bg-surface p-5">
        <h2 className="font-semibold text-fg-strong">Add freelancer</h2>
        <div className="mt-4 space-y-3">
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Name"
            aria-label="Freelancer name"
            className="w-full rounded-lg border border-line bg-canvas px-3 py-2 text-sm"
          />
          <input
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="Email"
            aria-label="Freelancer email"
            className="w-full rounded-lg border border-line bg-canvas px-3 py-2 text-sm"
          />
          <select
            value={status}
            aria-label="Freelancer status"
            onChange={(e) => setStatus(e.target.value)}
            className="w-full rounded-lg border border-line bg-canvas px-3 py-2 text-sm"
          >
            <option value="lead">Lead</option>
            <option value="active">Active</option>
            <option value="paused">Paused</option>
            <option value="archived">Archived</option>
          </select>
          <button
            type="button"
            disabled={busy || !name.trim()}
            onClick={() => void create()}
            className="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-foreground disabled:opacity-50"
          >
            Save freelancer
          </button>
        </div>
      </div>
    </section>
  );
}
