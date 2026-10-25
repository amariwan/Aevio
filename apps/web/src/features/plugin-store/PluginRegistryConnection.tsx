import { Cloud, KeyRound, Link2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";

type Props = {
  registryUrl: string;
  bearerToken: string;
  workspaceId: string;
  connected: boolean;
  busy: boolean;
  onRegistryUrlChange: (value: string) => void;
  onBearerTokenChange: (value: string) => void;
  onWorkspaceIdChange: (value: string) => void;
  onConnect: () => void;
  onDisconnect: () => void;
};

export function PluginRegistryConnection({
  registryUrl,
  bearerToken,
  workspaceId,
  connected,
  busy,
  onRegistryUrlChange,
  onBearerTokenChange,
  onWorkspaceIdChange,
  onConnect,
  onDisconnect,
}: Props) {
  return (
    <Card className="bg-surface">
      <CardHeader className="flex-row items-center justify-between space-y-0">
        <div className="flex items-center gap-2">
          <Cloud className="size-4 text-accent" />
          <CardTitle className="text-base">Plugin registry</CardTitle>
        </div>
        <span className="text-xs text-fg-muted">{connected ? "Connected" : "Local catalog"}</span>
      </CardHeader>
      <CardContent className="grid gap-2 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)_auto]">
        <label className="sr-only" htmlFor="plugin-registry-url">
          Registry URL
        </label>
        <div className="relative">
          <Link2 className="pointer-events-none absolute top-2 left-2.5 size-4 text-fg-faint" />
          <Input
            id="plugin-registry-url"
            value={registryUrl}
            onChange={(event) => onRegistryUrlChange(event.target.value)}
            placeholder="https://plugins.example.com"
            className="pl-8"
            disabled={busy}
          />
        </div>
        <label className="sr-only" htmlFor="plugin-workspace-id">
          Workspace ID
        </label>
        <Input
          id="plugin-workspace-id"
          value={workspaceId}
          onChange={(event) => onWorkspaceIdChange(event.target.value)}
          placeholder="Workspace ID (optional)"
          disabled={busy}
        />
        <label className="sr-only" htmlFor="plugin-registry-token">
          Registry bearer token
        </label>
        <div className="relative">
          <KeyRound className="pointer-events-none absolute top-2 left-2.5 size-4 text-fg-faint" />
          <Input
            id="plugin-registry-token"
            type="password"
            value={bearerToken}
            onChange={(event) => onBearerTokenChange(event.target.value)}
            placeholder="Bearer token (not stored)"
            className="pl-8"
            disabled={busy}
          />
        </div>
        {connected ? (
          <Button variant="outline" disabled={busy} onClick={onDisconnect}>
            Disconnect
          </Button>
        ) : (
          <Button disabled={busy || !registryUrl.trim() || !bearerToken.trim()} onClick={onConnect}>
            Connect
          </Button>
        )}
      </CardContent>
    </Card>
  );
}
