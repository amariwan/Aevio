import type { ReactNode } from "react";
import { motion } from "motion/react";

import { useI18n } from "@/hooks/useI18n";
import { useCatalog } from "@/hooks/useCatalog";

import { PLATFORM } from "@/lib/platform";
import type { Route } from "@/lib/types";
import { Tooltip } from "../primitives/Tooltip";
import { Picker } from "../primitives/Picker";
import { NotificationCenter } from "./NotificationCenter";

interface TopBarProps {
  canGoBack: boolean;
  canGoForward: boolean;
  onBack: () => void;
  onForward: () => void;
  sidebarCollapsed: boolean;
  onToggleSidebar: () => void;
  currentRoute: Route;
  onNavigate: (route: Route) => void;
  updateVersion?: string;
  pendingCount: number;
  breakReminder?: { kind: "break" | "stopwatch"; label: string };
  onOpenUpdate: () => void;
  onOpenReview: () => void;
  onOpenReminder: (kind: "break" | "stopwatch") => void;
  onOpenNotificationSettings: () => void;
}

/**
 * The app's own bar. On macOS `titleBarStyle: "Overlay"` (tauri.conf.json)
 * draws the webview under a transparent title bar, so this row *is* the title
 * bar and must leave room for the traffic lights on the left. Elsewhere it is a
 * normal toolbar under the native title bar and needs no inset.
 */
export function TopBar({
  canGoBack,
  canGoForward,
  onBack,
  onForward,
  sidebarCollapsed,
  onToggleSidebar,
  currentRoute,
  onNavigate,
  updateVersion,
  pendingCount,
  breakReminder,
  onOpenUpdate,
  onOpenReview,
  onOpenReminder,
  onOpenNotificationSettings,
}: TopBarProps) {
  const { t } = useI18n();
  const catalog = useCatalog();
  const selectedProjectId = currentRoute.name === "projects" ? (currentRoute.projectId ?? "") : "";
  const projectOptions = [
    { value: "", label: t("nav.projects") },
    ...catalog.projects
      .filter((project) => project.status !== "archived")
      .map((project) => ({ value: project.id, label: project.name, color: project.color })),
  ];
  const clientOptions = [
    { value: "", label: "Clients" },
    ...catalog.clients
      .filter((client) => !client.archivedAt)
      .map((client) => ({ value: client.id, label: client.name })),
  ];

  return (
    <header
      data-tauri-drag-region="deep"
      className={`flex h-11 shrink-0 items-center gap-2 border-b border-line bg-bar pr-3 ${
        PLATFORM === "macos" ? "pl-[84px]" : "pl-3"
      }`}
    >
      <div className="flex flex-1 items-center gap-2">
        <NavArrow
          label={t("common.back")}
          enabled={canGoBack}
          onClick={onBack}
          path="M15 5l-7 7 7 7"
        />
        <NavArrow
          label={t("common.forward")}
          enabled={canGoForward}
          onClick={onForward}
          path="M9 5l7 7-7 7"
        />
        <IconButton
          label={sidebarCollapsed ? t("common.expandSidebar") : t("common.collapseSidebar")}
          onClick={onToggleSidebar}
          large
        >
          <rect x="3" y="4" width="18" height="16" rx="2" />
          <path d="M9 4v16" />
          {sidebarCollapsed ? <path d="M14 10l2 2-2 2" /> : <path d="M16 10l-2 2 2 2" />}
        </IconButton>
      </div>

      <div className="shrink-0">
        <span className="select-none font-mono text-[11.5px] font-semibold tracking-wide text-fg-soft">
          Aevio
        </span>
      </div>

      <div className="flex flex-1 items-center justify-end gap-1.5">
        <Picker<string>
          ariaLabel={t("common.chooseProject")}
          value={selectedProjectId}
          options={projectOptions}
          onChange={(projectId) =>
            onNavigate(projectId === "" ? { name: "projects" } : { name: "projects", projectId })
          }
          variant="compact"
          buttonClassName="max-w-36"
        />
        <Picker<string>
          ariaLabel="Client"
          value=""
          options={clientOptions}
          onChange={(clientId) =>
            onNavigate(clientId === "" ? { name: "clients" } : { name: "clients", clientId })
          }
          variant="compact"
          buttonClassName="max-w-36"
        />
        <NotificationCenter
          updateVersion={updateVersion}
          pendingCount={pendingCount}
          reminder={breakReminder}
          onOpenUpdates={onOpenUpdate}
          onOpenReview={onOpenReview}
          onOpenReminder={onOpenReminder}
          onOpenSettings={onOpenNotificationSettings}
        />
      </div>
    </header>
  );
}

function NavArrow({
  label,
  path,
  enabled,
  onClick,
}: {
  label: string;
  path: string;
  enabled: boolean;
  onClick: () => void;
}) {
  return (
    <IconButton label={label} disabled={!enabled} onClick={onClick}>
      <path d={path} />
    </IconButton>
  );
}

function IconButton({
  label,
  disabled = false,
  large = false,
  onClick,
  children,
}: {
  label: string;
  disabled?: boolean;
  large?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <Tooltip content={label}>
      <motion.button
        type="button"
        aria-label={label}
        disabled={disabled}
        onClick={onClick}
        className="grid size-7 shrink-0 place-items-center rounded-lg text-fg-soft hover:bg-surface-strong disabled:pointer-events-none disabled:text-fg-ghost"
        whileHover={disabled ? undefined : { y: -1 }}
        whileTap={disabled ? undefined : { scale: 0.94 }}
        transition={{ type: "spring", stiffness: 400, damping: 28 }}
      >
        <svg
          viewBox="0 0 24 24"
          className={large ? "size-[18px]" : "size-3.5"}
          fill="none"
          stroke="currentColor"
          strokeWidth={2.2}
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          {children}
        </svg>
      </motion.button>
    </Tooltip>
  );
}
