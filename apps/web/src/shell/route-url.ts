import type { Route } from "@/lib/types";

const SEGMENTS: Record<Route["name"], string> = {
  dashboard: "dashboard",
  initiatives: "initiatives",
  notes: "notes",
  documents: "documents",
  issues: "issues",
  focus: "focus",
  agent: "agent",
  finance: "finance",
  staffing: "staffing",
  coach: "coach",
  tasks: "tasks",
  teams: "teams",
  calendar: "calendar",
  timesheet: "timesheet",
  timers: "timers",
  apps: "apps",
  entries: "time-entries",
  timesheets: "timesheets",
  projects: "projects",
  clients: "clients",
  invoices: "invoices",
  settings: "settings",
};

const SEGMENT_TO_NAME: Record<string, Route["name"]> = Object.fromEntries(
  Object.entries(SEGMENTS).map(([name, seg]) => [seg, name as Route["name"]]),
) as Record<string, Route["name"]>;

export function routeToSegment(name: Route["name"]): string {
  return SEGMENTS[name];
}

export function segmentToRouteName(segment: string): Route["name"] | undefined {
  return SEGMENT_TO_NAME[segment];
}

export function routeToUrl(route: Route, lang: string): string {
  const seg = SEGMENTS[route.name];
  const params = new URLSearchParams();
  switch (route.name) {
    case "calendar":
      if (route.scale) params.set("scale", route.scale);
      if (route.date) params.set("date", route.date);
      if (route.entryId) params.set("entryId", route.entryId);
      if (route.review) params.set("review", "1");
      break;
    case "timesheet":
      if (route.scale) params.set("scale", route.scale);
      if (route.date) params.set("date", route.date);
      if (route.tab) params.set("tab", route.tab);
      if (route.groupBy) params.set("groupBy", route.groupBy);
      break;
    case "apps":
      if (route.scale) params.set("scale", route.scale);
      if (route.date) params.set("date", route.date);
      if (route.tab) params.set("tab", route.tab);
      if (route.appId) params.set("appId", route.appId);
      break;
    case "entries":
      if (route.range) params.set("range", route.range);
      if (route.date) params.set("date", route.date);
      if (route.view) params.set("view", route.view);
      if (route.groupBy) params.set("groupBy", route.groupBy);
      if (route.stackBy) params.set("stackBy", route.stackBy);
      if (route.filters) {
        const f = route.filters;
        if (f.categoryId) params.set("categoryId", f.categoryId);
        if (f.projectId) params.set("projectId", f.projectId);
        if (f.clientId) params.set("clientId", f.clientId);
        if (f.app) params.set("app", f.app);
        if (f.status) params.set("status", f.status);
        if (f.billable !== undefined) params.set("billable", String(f.billable));
        if (f.search) params.set("search", f.search);
      }
      break;
    case "timesheets":
      if (route.scale) params.set("scale", route.scale);
      if (route.date) params.set("date", route.date);
      if (route.rows) params.set("rows", route.rows);
      if (route.filters) {
        const f = route.filters;
        if (f.clientIds?.length) params.set("clientIds", f.clientIds.join(","));
        if (f.projectIds?.length) params.set("projectIds", f.projectIds.join(","));
        if (f.categoryIds?.length) params.set("categoryIds", f.categoryIds.join(","));
      }
      break;
    case "projects":
      if (route.tab) params.set("tab", route.tab);
      if (route.projectId) params.set("projectId", route.projectId);
      if (route.range) params.set("range", route.range);
      break;
    case "clients":
      if (route.tab) params.set("tab", route.tab);
      if (route.clientId) params.set("clientId", route.clientId);
      if (route.range) params.set("range", route.range);
      break;
    case "invoices":
      if (route.invoiceId) params.set("invoiceId", route.invoiceId);
      if (route.compose) {
        if (route.compose.clientId) params.set("composeClient", route.compose.clientId);
        if (route.compose.projectId) params.set("composeProject", route.compose.projectId);
      }
      break;
    case "settings":
      if (route.section) params.set("section", route.section);
      break;
    case "teams":
      if (route.tab) params.set("tab", route.tab);
      break;
    default:
      break;
  }
  const qs = params.toString();
  return `/${lang}/${seg}/${qs ? `?${qs}` : ""}`;
}

export function urlToRoute(pathname: string, search: URLSearchParams): Route {
  const segments = pathname.split("/").filter(Boolean);
  // pathname like /de/dashboard or /de/time-entries
  const viewSeg = segments.length >= 2 ? segments[1] : (segments[0] ?? "dashboard");
  const name = segmentToRouteName(viewSeg) ?? "dashboard";
  const g = (k: string) => search.get(k) ?? undefined;
  const gb = (k: string) => search.get(k) === "1" || search.get(k) === "true";
  switch (name) {
    case "calendar":
      return {
        name,
        scale: (g("scale") as "day" | "week" | "month" | undefined) ?? undefined,
        date: g("date"),
        entryId: g("entryId"),
        review: gb("review") || undefined,
      };
    case "timesheet":
      return {
        name,
        scale: (g("scale") as "day" | "week" | "month" | undefined) ?? undefined,
        date: g("date"),
        tab: (g("tab") as "review" | "processing" | "approved" | "all" | undefined) ?? undefined,
        groupBy: (g("groupBy") as "project" | "category" | "app" | "none" | undefined) ?? undefined,
      };
    case "apps":
      return {
        name,
        scale: (g("scale") as "day" | "week" | "month" | undefined) ?? undefined,
        date: g("date"),
        tab: (g("tab") as "timeline" | "log" | undefined) ?? undefined,
        appId: g("appId"),
      };
    case "entries": {
      const filters: Record<string, unknown> = {};
      const categoryId = g("categoryId");
      const projectId = g("projectId");
      const clientId = g("clientId");
      const app = g("app");
      const status = g("status");
      const billable = g("billable");
      const searchQ = g("search");
      if (categoryId) filters.categoryId = categoryId;
      if (projectId) filters.projectId = projectId;
      if (clientId) filters.clientId = clientId;
      if (app) filters.app = app;
      if (status) filters.status = status;
      if (billable !== undefined) filters.billable = billable === "true";
      if (searchQ) filters.search = searchQ;
      const hasFilters = Object.keys(filters).length > 0;
      return {
        name,
        range: (g("range") as "day" | "week" | "month" | "30d" | "year" | undefined) ?? undefined,
        date: g("date"),
        view: (g("view") as "table" | "charts" | "log" | undefined) ?? undefined,
        groupBy:
          (g("groupBy") as "project" | "client" | "category" | "app" | "status" | undefined) ??
          undefined,
        stackBy: (g("stackBy") as "day" | "week" | "month" | undefined) ?? undefined,
        filters: hasFilters ? (filters as never) : undefined,
      };
    }
    case "timesheets": {
      const clientIds = g("clientIds");
      const projectIds = g("projectIds");
      const categoryIds = g("categoryIds");
      const hasFilters = clientIds || projectIds || categoryIds;
      return {
        name,
        scale: (g("scale") as "day" | "week" | undefined) ?? undefined,
        date: g("date"),
        rows: (g("rows") as "project" | "category" | undefined) ?? undefined,
        filters: hasFilters
          ? {
              clientIds: clientIds ? clientIds.split(",") : undefined,
              projectIds: projectIds ? projectIds.split(",") : undefined,
              categoryIds: categoryIds ? categoryIds.split(",") : undefined,
            }
          : undefined,
      };
    }
    case "projects":
      return {
        name,
        tab: (g("tab") as "active" | "completed" | "archived" | undefined) ?? undefined,
        projectId: g("projectId"),
        range: (g("range") as "week" | "month" | "30d" | "all" | undefined) ?? undefined,
      };
    case "clients":
      return {
        name,
        tab: (g("tab") as "active" | "archived" | undefined) ?? undefined,
        clientId: g("clientId"),
        range: (g("range") as "week" | "month" | "30d" | "all" | undefined) ?? undefined,
      };
    case "invoices":
      return {
        name,
        invoiceId: g("invoiceId"),
        compose:
          g("composeClient") || g("composeProject")
            ? { clientId: g("composeClient"), projectId: g("composeProject") }
            : undefined,
      };
    case "settings":
      return { name, section: g("section") };
    case "teams":
      return {
        name,
        tab:
          (g("tab") as "teams" | "members" | "departments" | "defaults" | "audit" | undefined) ??
          undefined,
      };
    default:
      return { name } as Route;
  }
}

export function langFromPathname(pathname: string): string {
  const seg = pathname.split("/").filter(Boolean)[0];
  return seg === "de" || seg === "en" ? seg : "en";
}
