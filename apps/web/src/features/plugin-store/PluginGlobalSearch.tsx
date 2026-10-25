import { AlertTriangle, ArrowUpRight, Search } from "lucide-react";
import { type FormEvent, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import * as api from "@/lib/api";
import type { PluginSearchResult, Route } from "@/lib/types";

type Props = { onNavigate: (route: Route) => void };

function routeForTarget(target: string): Route | undefined {
  const route = target.split("/")[0];
  const routes: Record<string, Route> = {
    calendar: { name: "calendar" },
    clients: { name: "clients" },
    notes: { name: "notes" },
    projects: { name: "projects" },
    timers: { name: "timers" },
  };
  return routes[route];
}

export function PluginGlobalSearch({ onNavigate }: Props) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<PluginSearchResult[]>([]);
  const [failedProviders, setFailedProviders] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const normalized = query.trim();
    if (!normalized) {
      setResults([]);
      setFailedProviders([]);
      return;
    }
    try {
      setBusy(true);
      setError(undefined);
      const aggregate = await api.pluginSearchAll(normalized);
      setResults(aggregate.results);
      setFailedProviders(aggregate.failures.map((failure) => failure.pluginId));
    } catch (cause: unknown) {
      setError(api.describeError(cause));
      setResults([]);
      setFailedProviders([]);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card className="bg-surface">
      <CardHeader className="pb-3">
        <CardTitle className="flex items-center gap-2 text-base">
          <Search className="size-4 text-accent" /> Plugin search
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <form className="flex gap-2" onSubmit={(event) => void submit(event)}>
          <Input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search active plugin providers"
            aria-label="Search active plugin providers"
          />
          <Button type="submit" disabled={busy || query.trim().length === 0}>
            {busy ? "Searching…" : "Search"}
          </Button>
        </form>
        {error && <p className="text-sm text-danger">{error}</p>}
        {failedProviders.length > 0 && (
          <p className="flex items-center gap-1 text-xs text-fg-muted">
            <AlertTriangle className="size-3.5 text-warning" />
            {failedProviders.length} provider(s) unavailable; results are partial.
          </p>
        )}
        {results.length > 0 && (
          <ul className="divide-y divide-line rounded-lg border border-line">
            {results.map((result) => {
              const route = routeForTarget(result.target);
              return (
                <li key={`${result.pluginId}:${result.target}`}>
                  <button
                    type="button"
                    disabled={!route}
                    onClick={() => route && onNavigate(route)}
                    className="flex w-full items-center gap-3 px-3 py-2 text-left hover:bg-surface-strong disabled:cursor-default disabled:opacity-60"
                  >
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-medium text-fg">
                        {result.title}
                      </span>
                      <span className="block truncate text-xs text-fg-muted">{result.detail}</span>
                    </span>
                    {route && <ArrowUpRight className="size-4 shrink-0 text-accent" />}
                  </button>
                </li>
              );
            })}
          </ul>
        )}
        {!busy && query.trim() && results.length === 0 && !error && (
          <p className="text-sm text-fg-muted">No active plugin provider returned a result.</p>
        )}
      </CardContent>
    </Card>
  );
}
