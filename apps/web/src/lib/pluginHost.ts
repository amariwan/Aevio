import { invoke, listen } from "./tauri";
import { createTauriPluginHostTransport, type PluginHostTransport } from "@aevio/plugin-sdk";

/**
 * Host transport for trusted first-party/plugin runtime adapters.
 * Third-party UI is never dynamically imported into this React window; this
 * object only exposes the SDK's capability-shaped transport.
 */
export const pluginHostTransport: PluginHostTransport = createTauriPluginHostTransport(
  <T>(command: string, args: Record<string, unknown>) => invoke<T>(command, args),
  async <T>(event: string, handler: (payload: T) => void) => {
    const unlisten = await listen<T>(event, (message) => handler(message.payload));
    return unlisten;
  },
);
