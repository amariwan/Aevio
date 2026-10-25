import { isTauri as tauriIsTauri } from "@tauri-apps/api/core";

/** Tauri's runtime marker works both with and without `withGlobalTauri`. */

export function isTauri(): boolean {
  return typeof window !== "undefined" && tauriIsTauri();
}

/**
 * Guarded `invoke`. Throws when not in Tauri so callers' `.catch()` handlers
 * can suppress the error.
 */
export async function invoke<T>(
  cmd: string,
  args?: import("@tauri-apps/api/core").InvokeArgs,
): Promise<T> {
  if (!isTauri()) {
    throw new Error(`Tauri unavailable: cannot invoke ${cmd}`);
  }
  const { invoke: tauriInvoke } = await import("@tauri-apps/api/core");
  return tauriInvoke<T>(cmd, args);
}

/**
 * Guarded `listen`. Returns a no-op unlisten when not in Tauri.
 */
export async function listen<T>(
  event: string,
  handler: (message: { payload: T }) => void,
): Promise<() => void> {
  if (!isTauri()) {
    return () => {};
  }
  const { listen: tauriListen } = await import("@tauri-apps/api/event");
  return tauriListen<T>(event, handler);
}
