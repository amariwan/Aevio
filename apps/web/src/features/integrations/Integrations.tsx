import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useI18n } from "@/hooks/useI18n";
import { ArrowUpRight, Check, Link2, LockKeyhole, Plus, RefreshCw, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import * as api from "@/lib/api";
import type { IntegrationConnection, IntegrationDirection, IntegrationProvider } from "@/lib/types";

type Category =
  | "Professional service automation"
  | "Calendars"
  | "Communication"
  | "Billing tools"
  | "Workflow & productivity"
  | "Storage & file share"
  | "Staffing & Freelancers";

interface ProviderCard {
  id: string;
  name: string;
  category: Category;
  description: string;
  provider?: IntegrationProvider;
  accent: string;
}

const PROVIDERS: readonly ProviderCard[] = [
  {
    id: "moco",
    name: "MOCO",
    category: "Professional service automation",
    description: "Projects, clients and billable work",
    accent: "bg-accent-soft",
  },
  {
    id: "google-calendar",
    name: "Google Calendar",
    category: "Calendars",
    description: "Meetings and availability",
    accent: "bg-success-soft",
  },
  {
    id: "microsoft-calendar",
    name: "Microsoft 365",
    category: "Calendars",
    description: "Outlook calendar and meetings",
    accent: "bg-accent-soft",
  },
  {
    id: "slack",
    name: "Slack",
    category: "Communication",
    description: "Team context and notifications",
    accent: "bg-warning-soft",
  },
  {
    id: "stripe",
    name: "Stripe",
    category: "Billing tools",
    description: "Payments and billing events",
    accent: "bg-accent-soft",
  },
  {
    id: "clickup",
    name: "ClickUp",
    category: "Workflow & productivity",
    description: "Tasks, labels and tracked time",
    provider: "clickup",
    accent: "bg-review-soft",
  },
  {
    id: "linear",
    name: "Linear",
    category: "Workflow & productivity",
    description: "Issues and project delivery",
    provider: "linear",
    accent: "bg-success-soft",
  },
  {
    id: "asana",
    name: "Asana",
    category: "Workflow & productivity",
    description: "Tasks, projects and time",
    provider: "asana",
    accent: "bg-warning-soft",
  },
  {
    id: "drive",
    name: "Google Drive",
    category: "Storage & file share",
    description: "Files, folders and shared work",
    accent: "bg-success-soft",
  },
  {
    id: "dropbox",
    name: "Dropbox",
    category: "Storage & file share",
    description: "Shared files and client assets",
    accent: "bg-accent-soft",
  },
  {
    id: "upwork",
    name: "Upwork",
    category: "Staffing & Freelancers",
    description: "Freelancer capacity and contracts",
    accent: "bg-review-soft",
  },
  {
    id: "personio",
    name: "Personio",
    category: "Staffing & Freelancers",
    description: "People operations and staffing",
    accent: "bg-warning-soft",
  },
];

const CATEGORIES: readonly Category[] = [
  "Professional service automation",
  "Calendars",
  "Communication",
  "Billing tools",
  "Workflow & productivity",
  "Storage & file share",
  "Staffing & Freelancers",
];

const PROVIDER_LABELS: Record<IntegrationProvider, string> = {
  clickup: "ClickUp",
  linear: "Linear",
  asana: "Asana",
};

export function Integrations() {
  const [connections, setConnections] = useState<IntegrationConnection[]>([]);
  const [selected, setSelected] = useState<ProviderCard | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async (): Promise<void> => {
    setLoading(true);
    setError(null);
    try {
      setConnections(await api.listIntegrations());
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  const connectedProviders = useMemo(
    () => new Set(connections.map((connection) => connection.provider)),
    [connections],
  );

  const save = async (input: ConnectionDraft): Promise<void> => {
    if (!selected?.provider) return;
    setSaving(true);
    setError(null);
    const now = Date.now();
    const existing = connections.find(
      (connection) =>
        connection.provider === selected.provider && connection.workspaceRef === input.workspaceRef,
    );
    try {
      const saved = await api.saveIntegration({
        id: existing?.id ?? crypto.randomUUID(),
        provider: selected.provider,
        name: input.name.trim(),
        workspaceRef: input.workspaceRef.trim(),
        secretRef: input.secretRef.trim(),
        direction: input.direction,
        enabled: true,
        createdAt: existing?.createdAt ?? now,
        updatedAt: now,
      });
      setConnections((current) => [
        ...current.filter((connection) => connection.id !== saved.id),
        saved,
      ]);
      setSelected(null);
    } catch (cause) {
      setError(api.describeError(cause));
    } finally {
      setSaving(false);
    }
  };

  return (
    <main className="min-h-0 flex-1 overflow-y-auto bg-canvas px-6 py-7 text-fg">
      <div className="mx-auto flex max-w-6xl flex-col gap-7">
        <header className="flex flex-wrap items-end justify-between gap-5">
          <div className="max-w-2xl">
            <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-fg-faint">
              Workspace
            </p>
            <h1 className="mt-2 text-3xl font-semibold tracking-tight text-fg-strong">
              Connect your stack
            </h1>
            <p className="mt-2 text-sm leading-6 text-fg-muted">
              Bring the tools your team already trusts into AEVIO. Keep work, context and billable
              time connected in one place.
            </p>
          </div>
          <div className="flex items-center gap-2 text-xs text-fg-muted">
            <span className="flex size-8 items-center justify-center rounded-full bg-accent-soft text-accent">
              <Link2 className="size-4" />
            </span>
            <span>
              <strong className="text-fg-strong">{connections.length}</strong> connected
            </span>
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => void load()}
              disabled={loading}
              aria-label="Refresh integrations"
            >
              <RefreshCw className={loading ? "size-4 animate-spin" : "size-4"} />
            </Button>
          </div>
        </header>

        {error && (
          <div
            role="alert"
            className="rounded-lg border border-danger/30 bg-danger-soft px-3 py-2 text-sm text-danger"
          >
            {error}
          </div>
        )}

        {connections.length > 0 && (
          <section className="flex flex-col gap-3">
            <SectionHeading
              title="Connected tools"
              description="Durable connection metadata is stored locally. Provider secrets stay in the OS keychain."
            />
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
              {connections.map((connection) => (
                <ConnectedCard key={connection.id} connection={connection} />
              ))}
            </div>
          </section>
        )}

        <section className="flex flex-col gap-5">
          <SectionHeading
            title="Explore integrations"
            description="Choose a tool to start a secure connection setup. Available providers are ready for metadata and sync configuration."
          />
          <div className="flex flex-col gap-8">
            {CATEGORIES.map((category) => (
              <div key={category}>
                <div className="mb-3 flex items-center gap-3">
                  <h2 className="text-sm font-semibold text-fg-strong">{category}</h2>
                  <span className="h-px flex-1 bg-line" />
                </div>
                <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
                  {PROVIDERS.filter((provider) => provider.category === category).map(
                    (provider) => (
                      <ProviderCardView
                        key={provider.id}
                        card={provider}
                        connected={
                          provider.provider ? connectedProviders.has(provider.provider) : false
                        }
                        onSelect={setSelected}
                      />
                    ),
                  )}
                </div>
              </div>
            ))}
          </div>
        </section>
      </div>
      {selected && (
        <ConnectionPanel
          card={selected}
          saving={saving}
          onClose={() => setSelected(null)}
          onSave={save}
        />
      )}
    </main>
  );
}

interface ConnectionDraft {
  name: string;
  workspaceRef: string;
  secretRef: string;
  direction: IntegrationDirection;
}

function ConnectionPanel({
  card,
  saving,
  onClose,
  onSave,
}: {
  card: ProviderCard;
  saving: boolean;
  onClose: () => void;
  onSave: (draft: ConnectionDraft) => Promise<void>;
}) {
  const { t } = useI18n();
  const [draft, setDraft] = useState<ConnectionDraft>({
    name: card.name,
    workspaceRef: "",
    secretRef: "",
    direction: "bidirectional",
  });
  const canSave =
    draft.name.trim() !== "" && draft.workspaceRef.trim() !== "" && draft.secretRef.trim() !== "";
  const update = (patch: Partial<ConnectionDraft>): void =>
    setDraft((current) => ({ ...current, ...patch }));
  return (
    <div className="fixed inset-0 z-40 flex items-end justify-center bg-scrim/25 p-4 sm:items-center">
      <Card className="w-full max-w-lg shadow-xl">
        <CardHeader className="flex-row items-start justify-between gap-4">
          <div>
            <CardTitle>Connect {card.name}</CardTitle>
            <CardDescription className="mt-1">
              Save the connection metadata used by AEVIO.
            </CardDescription>
          </div>
          <Button variant="ghost" size="icon-sm" onClick={onClose} aria-label="Close">
            <X className="size-4" />
          </Button>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <Field label="Connection name">
            <Input value={draft.name} onChange={(event) => update({ name: event.target.value })} />
          </Field>
          <Field label="Workspace or account reference">
            <Input
              placeholder={t("integrations.workspace")}
              value={draft.workspaceRef}
              onChange={(event) => update({ workspaceRef: event.target.value })}
            />
          </Field>
          <Field label="OS keychain reference">
            <Input
              placeholder={t("integrations.keychain")}
              value={draft.secretRef}
              onChange={(event) => update({ secretRef: event.target.value })}
            />
            <p className="mt-1 flex items-center gap-1 text-[11px] text-fg-muted">
              <LockKeyhole className="size-3" />
              Never paste an access token here. Only a keychain reference is stored.
            </p>
          </Field>
          <Field label="Sync direction">
            <select
              value={draft.direction}
              onChange={(event) =>
                update({ direction: event.target.value as IntegrationDirection })
              }
              className="h-9 rounded-md border border-input bg-surface px-3 text-sm text-fg-strong"
            >
              <option value="bidirectional">Two-way sync</option>
              <option value="pull">Pull into AEVIO</option>
              <option value="push">Push from AEVIO</option>
            </select>
          </Field>
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button onClick={() => void onSave(draft)} disabled={!canSave || saving}>
              {saving ? "Saving…" : "Save connection"}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="text-sm font-medium text-fg-strong">
      <span className="mb-1.5 block">{label}</span>
      {children}
    </label>
  );
}

function SectionHeading({ title, description }: { title: string; description: string }) {
  return (
    <header>
      <h2 className="text-lg font-semibold text-fg-strong">{title}</h2>
      <p className="mt-1 text-sm text-fg-muted">{description}</p>
    </header>
  );
}

function ProviderCardView({
  card,
  connected,
  onSelect,
}: {
  card: ProviderCard;
  connected: boolean;
  onSelect: (card: ProviderCard) => void;
}) {
  const available = card.provider !== undefined;
  return (
    <Card className="group transition-shadow hover:shadow-md">
      <CardContent className="flex items-start gap-3 p-4">
        <div
          className={`flex size-10 shrink-0 items-center justify-center rounded-xl text-sm font-bold text-fg-strong ${card.accent}`}
        >
          {card.name.slice(0, 1)}
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h3 className="truncate text-sm font-semibold text-fg-strong">{card.name}</h3>
            {connected && (
              <span className="flex items-center gap-1 text-[11px] font-medium text-success">
                <Check className="size-3" />
                Connected
              </span>
            )}
          </div>
          <p className="mt-1 text-xs leading-5 text-fg-muted">{card.description}</p>
          <Button
            className="mt-3"
            variant={available ? "outline" : "ghost"}
            size="sm"
            disabled={!available}
            onClick={() => onSelect(card)}
          >
            {connected ? (
              "Manage"
            ) : available ? (
              <>
                <Plus className="mr-1 size-3.5" />
                Connect
              </>
            ) : (
              "Coming soon"
            )}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function ConnectedCard({ connection }: { connection: IntegrationConnection }) {
  return (
    <Card className="border-success/30 bg-success-soft/30">
      <CardContent className="flex items-center gap-3 p-4">
        <div className="flex size-10 items-center justify-center rounded-xl bg-success-soft text-success">
          <Check className="size-5" />
        </div>
        <div className="min-w-0 flex-1">
          <h3 className="truncate text-sm font-semibold text-fg-strong">{connection.name}</h3>
          <p className="mt-1 truncate text-xs text-fg-muted">
            {PROVIDER_LABELS[connection.provider]} · {connection.workspaceRef}
          </p>
        </div>
        <ArrowUpRight className="size-4 text-fg-faint" />
      </CardContent>
    </Card>
  );
}
