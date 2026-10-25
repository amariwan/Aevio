/** Public, versioned contract for plugins running inside the Aevio host. */
export const PLUGIN_SDK_VERSION = "1.0.0" as const;
export const PLUGIN_MANIFEST_VERSION = 1 as const;

export const CAPABILITIES = {
  storageRead: "storage:own:read",
  storageWrite: "storage:own:write",
  projectsRead: "projects:read",
  projectsWrite: "projects:write",
  tasksRead: "tasks:read",
  tasksWrite: "tasks:write",
  notificationsShow: "notifications:show",
  filesReadSelected: "files:read-selected",
  filesWriteSelected: "files:write-selected",
  networkFetch: "network:fetch",
  searchContribute: "search:contribute",
  commandsRegister: "commands:register",
  widgetsRegister: "widgets:register",
  eventsSubscribe: "events:subscribe",
  eventsPublish: "events:publish",
  jobsSchedule: "jobs:schedule",
} as const;

export type Capability = (typeof CAPABILITIES)[keyof typeof CAPABILITIES];
export type PluginScope = "personal" | "workspace";

export interface PluginManifest {
  manifestVersion: typeof PLUGIN_MANIFEST_VERSION;
  id: string;
  name: string;
  version: string;
  publisher: { id: string; displayName: string };
  description: string;
  category: string;
  compatibility: { aevio: string; pluginApi: string };
  platforms: Array<"desktop" | "web" | "macos" | "windows" | "linux">;
  icons?: Array<{ path: string; mimeType?: string }>;
  screenshots?: Array<{ path: string; mimeType?: string }>;
  permissions?: Array<{ id: Capability; scope?: string; reason?: string }>;
  dependencies?: Array<{ pluginId: string; version: string }>;
  optionalDependencies?: Array<{ pluginId: string; version: string }>;
  eventSubscriptions?: Array<{
    id: string;
    eventType?: string;
    workflowId?: string;
    requiredCapabilities?: Capability[];
  }>;
  contributes?: PluginContributions;
}

export interface PluginContributions {
  navigation?: Array<{ id: string; label: string; page: string; icon?: string }>;
  pages?: Array<{ id: string; title: string }>;
  widgets?: Array<{ id: string; title: string; description: string }>;
  commands?: Array<{
    id: string;
    title: string;
    description?: string;
    requiredCapabilities?: Capability[];
  }>;
  settings?: Array<{ id: string; title: string }>;
  search?: Array<{ id: string; label: string; scopes?: string[] }>;
  workflowActions?: Array<{ id: string; title: string; requiredCapabilities?: Capability[] }>;
  workflowTriggers?: Array<{ id: string; title: string; requiredCapabilities?: Capability[] }>;
  jobs?: Array<{ id: string; title: string; minIntervalMs: number }>;
}

type NavigationContribution = NonNullable<PluginContributions["navigation"]>[number];
type PageContribution = NonNullable<PluginContributions["pages"]>[number];
type WidgetContribution = NonNullable<PluginContributions["widgets"]>[number];
type SettingsContribution = NonNullable<PluginContributions["settings"]>[number];

export interface PluginNavigationContext {
  readonly contributions: readonly NavigationContribution[];
}

export interface PluginPagesContext {
  readonly contributions: readonly PageContribution[];
}

export interface PluginWidgetsContext {
  readonly contributions: readonly WidgetContribution[];
}

export interface PluginSettingsContext {
  readonly contributions: readonly SettingsContribution[];
}

/** Declared service capabilities; privileged service calls remain host APIs. */
export interface PluginServicesContext {
  readonly declaredCapabilities: readonly Capability[];
}

export interface PluginDefinition {
  manifest: PluginManifest;
  activate(context: PluginContext): void | Promise<void>;
  deactivate?(): void | Promise<void>;
}

export interface PluginContext {
  readonly pluginId: string;
  readonly scope: PluginScope;
  readonly navigation: PluginNavigationContext;
  readonly pages: PluginPagesContext;
  readonly widgets: PluginWidgetsContext;
  readonly settings: PluginSettingsContext;
  readonly services: PluginServicesContext;
  readonly storage: PluginStorage;
  readonly events: PluginEvents;
  readonly commands: PluginCommands;
  readonly notifications: PluginNotifications;
  readonly network: PluginNetwork;
  readonly jobs: PluginJobs;
  readonly workflows: PluginWorkflows;
}

export interface PluginStorage {
  get<T>(key: string): Promise<T | undefined>;
  set<T>(key: string, value: T): Promise<void>;
  delete(key: string): Promise<void>;
}

export interface PluginEvents {
  subscribe(event: string, handler: (payload: unknown) => void): () => void;
  publish(event: string, payload: unknown): Promise<void>;
}

export interface PluginCommands {
  register(command: {
    id: string;
    title: string;
    description?: string;
    execute: (input?: unknown) => unknown | Promise<unknown>;
  }): () => void;
}

export interface PluginNotifications {
  show(input: { title: string; body: string }): Promise<void>;
}

export interface PluginNetwork {
  fetch(input: { url: string; method?: "GET" | "POST"; body?: string }): Promise<{
    status: number;
    body: string;
  }>;
}

export interface PluginJobs {
  schedule(input: { jobId: string; intervalMs: number; payload?: unknown }): Promise<PluginJob>;
  cancel(jobId: string): Promise<void>;
}

export interface PluginWorkflowRun {
  id: string;
  pluginId: string;
  workflowId: string;
  trigger: string;
  idempotencyKey: string;
  status: string;
  attemptCount: number;
  maxAttempts: number;
  input: unknown;
  output?: unknown;
  lastError?: string;
  createdAt: number;
  completedAt?: number;
}

export interface PluginWorkflows {
  run(input: {
    workflowId: string;
    trigger?: string;
    idempotencyKey: string;
    input?: unknown;
  }): Promise<PluginWorkflowRun>;
  cancel(workflowId: string, idempotencyKey: string): Promise<void>;
}

export interface PluginJob {
  id: string;
  pluginId: string;
  jobId: string;
  intervalMs: number;
  nextRunAt: number;
  lastRunAt?: number;
  runCount: number;
  enabled: boolean;
}

/** Typed host operations supplied by the Aevio adapter, never by plugin code. */
export interface PluginHostTransport {
  storage: {
    get(input: { pluginId: string; scope: PluginScope; key: string }): Promise<unknown>;
    set(input: { pluginId: string; scope: PluginScope; key: string; value: unknown }): Promise<void>;
    delete(input: { pluginId: string; scope: PluginScope; key: string }): Promise<void>;
  };
  events: {
    subscribe(input: {
      pluginId: string;
      scope: PluginScope;
      event: string;
      handler: (payload: unknown) => void;
    }): () => void;
    publish(input: {
      pluginId: string;
      scope: PluginScope;
      event: string;
      payload: unknown;
    }): Promise<void>;
  };
  commands: {
    register(input: {
      pluginId: string;
      scope: PluginScope;
      command: {
        id: string;
        title: string;
        description?: string;
        execute: (input?: unknown) => unknown | Promise<unknown>;
      };
    }): () => void;
  };
  notifications: {
    show(input: { pluginId: string; scope: PluginScope; title: string; body: string }): Promise<void>;
  };
  network: {
    fetch(input: {
      pluginId: string;
      scope: PluginScope;
      request: { url: string; method?: "GET" | "POST"; body?: string };
    }): Promise<{ status: number; body: string }>;
  };
  jobs: {
    schedule(input: {
      pluginId: string;
      scope: PluginScope;
      jobId: string;
      intervalMs: number;
      payload: unknown;
    }): Promise<PluginJob>;
    cancel(input: { pluginId: string; scope: PluginScope; jobId: string }): Promise<void>;
  };
  workflows: {
    run(input: {
      pluginId: string;
      scope: PluginScope;
      workflowId: string;
      trigger: string;
      idempotencyKey: string;
      input: unknown;
    }): Promise<PluginWorkflowRun>;
    cancel(input: {
      pluginId: string;
      scope: PluginScope;
      workflowId: string;
      idempotencyKey: string;
    }): Promise<void>;
  };
}

export type PluginHostInvoke = <T>(
  command: string,
  args: Record<string, unknown>,
) => Promise<T>;

export type PluginHostListen = <T>(
  event: string,
  handler: (payload: T) => void,
) => Promise<() => void>;

/**
 * Adapts the narrow Tauri invoke/event primitives to the SDK transport.
 * Privileged surfaces use dedicated, capability-shaped host commands; this
 * factory never exposes a generic invoke function to a plugin context.
 */
export function createTauriPluginHostTransport(
  invoke: PluginHostInvoke,
  listen: PluginHostListen,
): PluginHostTransport {
  return {
    storage: {
      get: ({ pluginId, key }) => invoke("plugin_storage_get", { pluginId, key }),
      set: ({ pluginId, key, value }) =>
        invoke("plugin_storage_set", { pluginId, key, value }).then(() => undefined),
      delete: ({ pluginId, key }) =>
        invoke("plugin_storage_delete", { pluginId, key }).then(() => undefined),
    },
    events: {
      subscribe: ({ pluginId, event, handler }) => {
        const eventId = event.startsWith(`${pluginId}/`) ? event.slice(pluginId.length + 1) : event;
        const eventName = `plugin-event:${eventId}`;
        let stopped = false;
        let unlisten: (() => void) | undefined;
        void invoke("plugin_subscribe_event", { pluginId, event: eventId })
          .then(() => listen<unknown>(eventName, handler))
          .then((stop) => {
            if (stopped) stop();
            else unlisten = stop;
          })
          .catch(() => undefined);
        return () => {
          stopped = true;
          unlisten?.();
        };
      },
      publish: ({ pluginId, event, payload }) =>
        invoke("plugin_publish_event", { pluginId, event, payload }).then(() => undefined),
    },
    commands: {
      register: () => {
        throw new Error("dynamic command registration is unavailable in the declarative host");
      },
    },
    notifications: {
      show: ({ pluginId, title, body }) =>
        invoke("plugin_show_notification", { pluginId, title, body }).then(() => undefined),
    },
    network: {
      fetch: ({ pluginId, request }) =>
        invoke<{ status: number; body: string }>("plugin_network_fetch", {
          pluginId,
          request,
        }),
    },
    jobs: {
      schedule: ({ pluginId, scope: _scope, jobId, intervalMs, payload }) =>
        invoke<PluginJob>("plugin_schedule_job", {
          request: { pluginId, jobId, intervalMs, payload },
        }),
      cancel: ({ pluginId, jobId }) =>
        invoke("plugin_cancel_job", { pluginId, jobId }).then(() => undefined),
    },
    workflows: {
      run: ({ pluginId, scope: _scope, workflowId, trigger, idempotencyKey, input }) =>
        invoke<PluginWorkflowRun>("plugin_workflow_run", {
          request: { pluginId, workflowId, trigger, idempotencyKey, input },
        }),
      cancel: ({ pluginId, scope: _scope, workflowId, idempotencyKey }) =>
        invoke("plugin_workflow_cancel", { pluginId, workflowId, idempotencyKey }).then(
          () => undefined,
        ),
    },
  };
}

/**
 * The host supplies this object only after validating the manifest and grants.
 * Plugins receive capability-specific methods, never a raw Tauri invoke or DB
 * handle. Every implementation must enforce grants again at the host boundary.
 */
export interface PluginHostBridge {
  createContext(
    manifest: PluginManifest,
    scope: PluginScope,
  ): Promise<PluginContext>;
}

/**
 * Build the SDK context from a host-owned transport. The transport is created
 * only after manifest validation and grant checks; plugin code receives no
 * generic invoke function or database handle.
 */
export function createPluginHostBridge(transport: PluginHostTransport): PluginHostBridge {
  return {
    async createContext(manifest, scope) {
      validateManifest(manifest);
      const pluginId = manifest.id;
      return {
        pluginId,
        scope,
        navigation: { contributions: manifest.contributes?.navigation ?? [] },
        pages: { contributions: manifest.contributes?.pages ?? [] },
        widgets: { contributions: manifest.contributes?.widgets ?? [] },
        settings: { contributions: manifest.contributes?.settings ?? [] },
        services: {
          declaredCapabilities: (manifest.permissions ?? []).map((permission) => permission.id),
        },
        storage: {
          get: async <T>(key: string) => (await transport.storage.get({ pluginId, scope, key })) as T | undefined,
          set: (key, value) => transport.storage.set({ pluginId, scope, key, value }),
          delete: (key) => transport.storage.delete({ pluginId, scope, key }),
        },
        events: {
          subscribe: (event, handler) =>
            transport.events.subscribe({ pluginId, scope, event, handler }),
          publish: (event, payload) =>
            transport.events.publish({ pluginId, scope, event, payload }),
        },
        commands: {
          register: (command) => transport.commands.register({ pluginId, scope, command }),
        },
        notifications: {
          show: ({ title, body }) => transport.notifications.show({ pluginId, scope, title, body }),
        },
        network: {
          fetch: (request) => transport.network.fetch({ pluginId, scope, request }),
        },
        jobs: {
          schedule: ({ jobId, intervalMs, payload = {} }) =>
            transport.jobs.schedule({ pluginId, scope, jobId, intervalMs, payload }),
          cancel: (jobId) => transport.jobs.cancel({ pluginId, scope, jobId }),
        },
        workflows: {
          run: ({ workflowId, trigger = "manual", idempotencyKey, input = {} }) =>
            transport.workflows.run({
              pluginId,
              scope,
              workflowId,
              trigger,
              idempotencyKey,
              input,
            }),
          cancel: (workflowId, idempotencyKey) =>
            transport.workflows.cancel({ pluginId, scope, workflowId, idempotencyKey }),
        },
      };
    },
  };
}

export function definePlugin(definition: PluginDefinition): PluginDefinition {
  validateManifest(definition.manifest);
  return definition;
}

export function validateManifest(manifest: PluginManifest): void {
  if (manifest.manifestVersion !== PLUGIN_MANIFEST_VERSION) {
    throw new Error("Unsupported plugin manifest version");
  }
  if (!/^[a-z0-9._-]+$/.test(manifest.id) || manifest.id.length > 160) {
    throw new Error("Plugin id must be a lowercase namespaced identifier");
  }
  if (!manifest.name.trim() || manifest.name.length > 120) {
    throw new Error("Plugin name must contain 1-120 characters");
  }
  for (const permission of manifest.permissions ?? []) {
    if (!Object.values(CAPABILITIES).includes(permission.id)) {
      throw new Error(`Unsupported plugin capability: ${permission.id}`);
    }
  }
  for (const contribution of contributionIds(manifest)) {
    if (!contribution.startsWith(`${manifest.id}/`)) {
      throw new Error(`Contribution id must be namespaced: ${contribution}`);
    }
  }
}

function contributionIds(manifest: PluginManifest): string[] {
  const contributions = manifest.contributes;
  if (!contributions) return [];
  return [
    ...(contributions.navigation ?? []),
    ...(contributions.pages ?? []),
    ...(contributions.widgets ?? []),
    ...(contributions.commands ?? []),
    ...(contributions.settings ?? []),
    ...(contributions.search ?? []),
    ...(contributions.workflowActions ?? []),
    ...(contributions.workflowTriggers ?? []),
    ...(contributions.jobs ?? []),
  ].map((contribution) => contribution.id);
}
