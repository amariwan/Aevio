import { createContext, type ReactNode, useContext } from "react";
import { motion } from "motion/react";
import { useCatalog } from "@/hooks/useCatalog";
import { usePluginExtensions } from "@/hooks/usePluginExtensions";
import { useI18n } from "@/hooks/useI18n";
import type { Route } from "@/lib/types";
import { Tooltip } from "../primitives/Tooltip";

interface SidebarProps {
  currentRoute: Route;
  onNavigate: (route: Route) => void;
  pendingCount?: number;
  currentApp?: string;
  captureEnabled?: boolean;
  trackingActive?: boolean;
  onToggleCapture?: () => void;
  /** A break in progress: the status strip shows its countdown. */
  onBreak?: { label: string; remainingMs: number };
  /** The newer version ready to install, when there is one. */
  updateVersion?: string;
  onOpenUpdate?: () => void;
  /** Break/stopwatch reminder for the notification center. */
  breakReminder?: { kind: "break" | "stopwatch"; label: string };
  onOpenReview?: () => void;
  onOpenReminder?: (kind: "break" | "stopwatch") => void;
  onOpenNotificationSettings?: () => void;
  /** Icons only, without labels. */
  collapsed?: boolean;
}

const CollapsedContext = createContext(false);

/** A group's heading, or a divider between groups when the sidebar is icons only. */
function GroupLabel({ children, first }: { children: string; first?: boolean }) {
  if (useContext(CollapsedContext)) {
    return first ? null : <div className="mx-1 mb-1 border-t border-line" />;
  }
  return (
    <span className="px-2 pb-1 text-[10.5px] font-semibold tracking-wider text-fg-faint uppercase">
      {children}
    </span>
  );
}

interface NavTabProps {
  active: boolean;
  label: string;
  badge?: number;
  onSelect: () => void;
  children: ReactNode;
}

function NavTab({ active, label, badge, onSelect, children }: NavTabProps) {
  const collapsed = useContext(CollapsedContext);
  const hasBadge = badge !== undefined && badge > 0;
  return (
    <Tooltip content={label} placement="right">
      <motion.button
        type="button"
        aria-label={label}
        onClick={onSelect}
        aria-current={active ? "page" : undefined}
        className={`relative flex w-full items-center gap-2.5 rounded-[9px] px-2.5 py-2 text-left text-[13px] font-medium transition-colors ${
          collapsed ? "justify-center" : ""
        } ${
          active
            ? "bg-accent-soft text-fg-strong font-semibold"
            : "text-fg-soft hover:bg-surface hover:text-fg"
        }`}
        whileHover={{ x: collapsed ? 0 : 2 }}
        whileTap={{ scale: 0.98 }}
        transition={{ type: "spring", stiffness: 400, damping: 28 }}
      >
        <svg
          viewBox="0 0 24 24"
          className="size-4 shrink-0"
          fill="none"
          stroke="currentColor"
          strokeWidth={1.8}
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          {children}
        </svg>
        {!collapsed && <span className="min-w-0 flex-1 truncate">{label}</span>}
        {hasBadge &&
          (collapsed ? (
            <span className="absolute top-1 right-1 size-2 rounded-full bg-review" />
          ) : (
            <span className="ml-auto rounded-full bg-review/20 px-2 py-0.5 text-[11px] font-bold text-review">
              {badge}
            </span>
          ))}
      </motion.button>
    </Tooltip>
  );
}

export function Sidebar({
  currentRoute,
  onNavigate,
  pendingCount = 0,
  updateVersion,
  onOpenUpdate,
  collapsed = false,
}: SidebarProps) {
  const catalog = useCatalog();
  const { navigation: pluginNavigation } = usePluginExtensions();
  const { t } = useI18n();
  const currentName = currentRoute.name;
  const visibleProjects = catalog.projects.filter((project) => project.status !== "archived");

  return (
    <CollapsedContext.Provider value={collapsed}>
      <aside
        className={`flex h-full min-h-0 flex-col border-r border-line bg-rail py-3 select-none ${
          collapsed ? "px-2" : "px-3"
        }`}
      >
        <nav
          className={`flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto ${
            collapsed ? "" : "pr-1"
          }`}
        >
          {/* Track Group */}
          <div className="flex flex-col gap-0.5">
            <GroupLabel first>{t("nav.track")}</GroupLabel>
            <NavTab
              active={currentName === "dashboard"}
              label={t("nav.dashboard")}
              onSelect={() => onNavigate({ name: "dashboard" })}
            >
              <rect x="3" y="3" width="7" height="7" rx="1" />
              <rect x="14" y="3" width="7" height="7" rx="1" />
              <rect x="3" y="14" width="7" height="7" rx="1" />
              <rect x="14" y="14" width="7" height="7" rx="1" />
            </NavTab>
            <NavTab
              active={currentName === "focus"}
              label={t("nav.focus")}
              onSelect={() => onNavigate({ name: "focus" })}
            >
              <circle cx="12" cy="12" r="8.5" />
              <path d="M12 7v5l3 2" />
            </NavTab>
            <NavTab
              active={currentName === "agent"}
              label={t("nav.agent")}
              onSelect={() => onNavigate({ name: "agent" })}
            >
              <path d="M12 3a9 9 0 0 0-9 9c0 2.2.8 4.2 2.1 5.8L4 21l3.3-1.2A9 9 0 1 0 12 3Z" />
              <path d="M8 12h.01M12 12h.01M16 12h.01" />
            </NavTab>
            <NavTab
              active={currentName === "coach"}
              label={t("nav.coach")}
              onSelect={() => onNavigate({ name: "coach" })}
            >
              <path d="M12 3a7 7 0 0 0-4 12.75V19h8v-3.25A7 7 0 0 0 12 3Z" />
              <path d="M9 22h6M9 19h6" />
            </NavTab>
            <NavTab
              active={currentName === "calendar"}
              label={t("nav.calendar")}
              badge={pendingCount}
              onSelect={() => onNavigate({ name: "calendar" })}
            >
              <rect x="3" y="4" width="18" height="18" rx="2" />
              <path d="M16 2v4M8 2v4M3 10h18" />
            </NavTab>
            <NavTab
              active={currentName === "timesheet"}
              label={t("nav.myTimesheet")}
              onSelect={() => onNavigate({ name: "timesheet" })}
            >
              <path d="M9 5H7a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-2" />
              <rect x="9" y="3" width="6" height="4" rx="1" />
              <path d="M9 14l2 2 4-4" />
            </NavTab>
            <NavTab
              active={currentName === "apps"}
              label={t("nav.apps")}
              onSelect={() => onNavigate({ name: "apps" })}
            >
              <rect x="3" y="3" width="7" height="7" rx="1.5" />
              <rect x="14" y="3" width="7" height="7" rx="1.5" />
              <rect x="14" y="14" width="7" height="7" rx="1.5" />
              <rect x="3" y="14" width="7" height="7" rx="1.5" />
            </NavTab>
          </div>

          {/* Analyze Group */}
          <div className="flex flex-col gap-0.5">
            <GroupLabel>{t("nav.analyze")}</GroupLabel>
            <NavTab
              active={currentName === "entries"}
              label={t("nav.timeEntries")}
              onSelect={() => onNavigate({ name: "entries" })}
            >
              <circle cx="12" cy="12" r="10" />
              <polyline points="12 6 12 12 16 14" />
            </NavTab>
            <NavTab
              active={currentName === "timesheets"}
              label={t("nav.timesheets")}
              onSelect={() => onNavigate({ name: "timesheets" })}
            >
              <rect x="3" y="3" width="18" height="18" rx="2" />
              <path d="M3 9h18M3 15h18M9 3v18M15 3v18" />
            </NavTab>
            <NavTab
              active={currentName === "finance"}
              label={t("nav.finance")}
              onSelect={() => onNavigate({ name: "finance" })}
            >
              <path d="M4 19V5M4 19h16" />
              <path d="m7 15 3-4 3 2 5-6" />
            </NavTab>
            <NavTab
              active={currentName === "staffing"}
              label="Staffing & Freelancers"
              onSelect={() => onNavigate({ name: "staffing" })}
            >
              <circle cx="8" cy="8" r="3" />
              <circle cx="16" cy="8" r="3" />
              <path d="M3 20c.5-3 2.2-5 5-5s4.5 2 5 5M11 20c.5-3 2.2-5 5-5s4.5 2 5 5" />
            </NavTab>
          </div>

          {pluginNavigation.length > 0 && (
            <div className="flex flex-col gap-0.5">
              <GroupLabel>{t("nav.extensions")}</GroupLabel>
              {pluginNavigation.map((entry) => (
                <NavTab
                  key={entry.id}
                  active={currentRoute.name === entry.route.name}
                  label={entry.label}
                  onSelect={() => onNavigate(entry.route)}
                >
                  <path d="M4 7.5A2.5 2.5 0 0 1 6.5 5h11A2.5 2.5 0 0 1 20 7.5v9a2.5 2.5 0 0 1-2.5 2.5h-11A2.5 2.5 0 0 1 4 16.5z" />
                  <path d="M8 12h8M12 8v8" />
                </NavTab>
              ))}
            </div>
          )}

          {/* Work Group */}
          <div className="flex flex-col gap-0.5">
            <GroupLabel>{t("nav.work")}</GroupLabel>
            <NavTab
              active={currentName === "initiatives"}
              label={t("nav.initiatives")}
              onSelect={() => onNavigate({ name: "initiatives" })}
            >
              <path d="m12 3 2.7 5.5 6.1.9-4.4 4.3 1 6.1-5.4-2.9-5.4 2.9 1-6.1-4.4-4.3 6.1-.9z" />
            </NavTab>
            <NavTab
              active={currentName === "documents"}
              label={t("nav.documents")}
              onSelect={() => onNavigate({ name: "documents" })}
            >
              <path d="M6 3h8l4 4v14H6z" />
              <path d="M14 3v5h5M9 13h6M9 17h6" />
            </NavTab>
            <NavTab
              active={currentName === "issues"}
              label={t("nav.issues")}
              onSelect={() => onNavigate({ name: "issues" })}
            >
              <circle cx="12" cy="12" r="8.5" />
              <path d="M12 7v5l3 2" />
            </NavTab>
            {!collapsed && visibleProjects.length > 0 && (
              <div className="ml-6 flex flex-col gap-0.5 border-l border-line pl-2">
                {visibleProjects.map((project) => {
                  const active =
                    currentRoute.name === "projects" && currentRoute.projectId === project.id;
                  return (
                    <button
                      key={project.id}
                      type="button"
                      aria-current={active ? "page" : undefined}
                      onClick={() => onNavigate({ name: "projects", projectId: project.id })}
                      className={`flex min-w-0 items-center gap-2 rounded-md px-2 py-1.5 text-left text-[12px] transition-colors ${
                        active
                          ? "bg-accent-soft font-semibold text-fg-strong"
                          : "text-fg-muted hover:bg-surface hover:text-fg"
                      }`}
                    >
                      <span
                        aria-hidden="true"
                        className="size-2 shrink-0 rounded-full"
                        style={{ backgroundColor: project.color }}
                      />
                      <span className="min-w-0 truncate">{project.name}</span>
                    </button>
                  );
                })}
              </div>
            )}
            <NavTab
              active={currentName === "invoices"}
              label={t("nav.invoices")}
              onSelect={() => onNavigate({ name: "invoices" })}
            >
              <rect x="4" y="2" width="16" height="20" rx="2" />
              <line x1="8" y1="6" x2="16" y2="6" />
              <line x1="8" y1="10" x2="16" y2="10" />
              <line x1="8" y1="14" x2="12" y2="14" />
            </NavTab>
          </div>

          {/* Manual Group */}
          <div className="flex flex-col gap-0.5">
            <GroupLabel>{t("nav.admin")}</GroupLabel>
            <NavTab
              active={currentName === "teams"}
              label={t("nav.teamsGovernance")}
              onSelect={() => onNavigate({ name: "teams" })}
            >
              <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
              <circle cx="9" cy="7" r="4" />
              <path d="M22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75" />
            </NavTab>
          </div>

          {/* Manual Group */}
          <div className="flex flex-col gap-0.5">
            <GroupLabel>{t("nav.manual")}</GroupLabel>
          </div>
        </nav>

        {/* Bottom Pinned Controls */}
        <div className="mt-auto flex shrink-0 flex-col gap-2 pt-2 border-t border-line">
          {updateVersion !== undefined && onOpenUpdate && (
            <Tooltip content={`Aevio ${updateVersion} is available`}>
              <button
                type="button"
                onClick={onOpenUpdate}
                aria-label={t("sidebar.updateAvailable")}
                className={`flex w-full items-center gap-2.5 rounded-[9px] bg-success-soft px-2.5 py-2 text-left text-[13px] font-semibold text-success transition-colors hover:bg-success/20 ${
                  collapsed ? "justify-center" : ""
                }`}
              >
                <svg
                  viewBox="0 0 24 24"
                  className="size-4 shrink-0 fill-current"
                  aria-hidden="true"
                >
                  <path d="M12 2.5l2.3 6.6 6.7 2.4-6.7 2.4L12 20.5l-2.3-6.6L3 11.5l6.7-2.4z" />
                </svg>
                {!collapsed && (
                  <span className="min-w-0 flex-1 truncate">{t("sidebar.updateAvailable")}</span>
                )}
              </button>
            </Tooltip>
          )}
          <NavTab
            active={currentName === "settings"}
            label={t("nav.settings")}
            onSelect={() => onNavigate({ name: "settings" })}
          >
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
          </NavTab>
        </div>
      </aside>
    </CollapsedContext.Provider>
  );
}
