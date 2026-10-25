import { useState } from "react";
import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import * as api from "@/lib/api";

interface TimelineSyncControlsProps {
  workspaceId: string;
  onSynced: () => Promise<void>;
}

export function TimelineSyncControls({ workspaceId, onSynced }: TimelineSyncControlsProps) {
  const [serverUrl, setServerUrl] = useState("http://127.0.0.1:47832");
  const [bearerToken, setBearerToken] = useState("");
  const [deviceId, setDeviceId] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const [result, setResult] = useState<string>();

  const sync = async () => {
    if (!workspaceId.trim() || !bearerToken.trim() || !deviceId.trim()) {
      setError("Workspace, bearer token and device ID are required.");
      return;
    }
    try {
      setBusy(true);
      setError(undefined);
      const synced = await api.syncTimeline({
        serverUrl: serverUrl.trim(),
        bearerToken: bearerToken.trim(),
        deviceId: deviceId.trim(),
        workspaceId: workspaceId.trim(),
      });
      setResult(`${synced.applied} changes received, ${synced.accepted} sent.`);
      await onSynced();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-sm">Server sync</CardTitle>
      </CardHeader>
      <CardContent className="grid gap-3 md:grid-cols-4">
        <Input
          aria-label="Sync server URL"
          placeholder="https://sync.example.com"
          value={serverUrl}
          onChange={(event) => setServerUrl(event.target.value)}
        />
        <Input
          aria-label="Bearer token"
          placeholder="bearer token"
          type="password"
          value={bearerToken}
          onChange={(event) => setBearerToken(event.target.value)}
        />
        <Input
          aria-label="Device ID"
          placeholder="this device ID"
          value={deviceId}
          onChange={(event) => setDeviceId(event.target.value)}
        />
        <Button disabled={busy} onClick={() => void sync()}>
          <RefreshCw className={busy ? "size-4 animate-spin" : "size-4"} />
          {busy ? "Syncing…" : "Sync now"}
        </Button>
        {(error || result) && (
          <p
            className={
              error ? "text-sm text-review md:col-span-4" : "text-sm text-fg-muted md:col-span-4"
            }
          >
            {error || result}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
