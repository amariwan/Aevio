import { Search } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import type { PluginCatalogEntry, PluginInstallationSummary } from "@/lib/types";

function stateLabel(state: PluginInstallationSummary["state"]): string {
  return state.replaceAll("_", " ");
}

type CatalogProps = {
  entries: PluginCatalogEntry[];
  categories: string[];
  category: string;
  query: string;
  selectedId: string | undefined;
  onCategoryChange: (category: string) => void;
  onQueryChange: (query: string) => void;
  onSelect: (pluginId: string) => void;
};

export function PluginStoreCatalog({
  entries,
  categories,
  category,
  query,
  selectedId,
  onCategoryChange,
  onQueryChange,
  onSelect,
}: CatalogProps) {
  return (
    <section className="min-w-0 space-y-3">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
        <div className="relative min-w-0 flex-1 sm:min-w-56">
          <Search className="pointer-events-none absolute top-2 left-2.5 size-4 text-fg-faint" />
          <Input
            value={query}
            onChange={(event) => onQueryChange(event.target.value)}
            placeholder="Search plugins"
            className="pl-8"
          />
        </div>
        <p className="shrink-0 text-xs text-fg-faint">
          {entries.length} {entries.length === 1 ? "plugin" : "plugins"}
        </p>
      </div>
      <div className="flex max-w-full gap-1.5 overflow-x-auto pb-1">
        {categories.map((item) => (
          <Button
            key={item}
            size="sm"
            variant={category === item ? "secondary" : "ghost"}
            className="shrink-0 capitalize"
            aria-pressed={category === item}
            onClick={() => onCategoryChange(item)}
          >
            {item}
          </Button>
        ))}
      </div>
      {entries.length === 0 ? (
        <div className="rounded-xl border border-dashed border-line px-4 py-10 text-center">
          <p className="text-sm font-medium text-fg">No plugins found</p>
          <p className="mt-1 text-sm text-fg-muted">Try another search or category.</p>
        </div>
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {entries.map((entry) => {
            const active = entry.manifest.id === selectedId;
            return (
              <button
                key={entry.manifest.id}
                type="button"
                aria-pressed={active}
                onClick={() => onSelect(entry.manifest.id)}
                className={`rounded-xl text-left transition-colors ${active ? "ring-2 ring-accent" : "ring-1 ring-transparent hover:ring-line-strong"}`}
              >
                <Card size="sm" className="h-full bg-surface shadow-none hover:bg-surface-strong">
                  <CardHeader className="gap-2">
                    <div className="flex items-start justify-between gap-3">
                      <CardTitle className="text-sm">{entry.manifest.name}</CardTitle>
                      <Badge variant="secondary">{entry.manifest.category}</Badge>
                    </div>
                    <p className="text-xs text-fg-faint">
                      {entry.manifest.publisher.displayName} · v{entry.manifest.version}
                    </p>
                  </CardHeader>
                  <CardContent>
                    <p className="text-sm text-fg-muted">{entry.manifest.description}</p>
                    {entry.installed && (
                      <p className="mt-3 text-xs font-medium text-accent">
                        {stateLabel(entry.installed.state)}
                      </p>
                    )}
                  </CardContent>
                </Card>
              </button>
            );
          })}
        </div>
      )}
    </section>
  );
}
