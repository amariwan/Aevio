"use client";

import { useMemo, useState } from "react";
import { BellRingIcon, CheckCheckIcon, Settings2Icon } from "lucide-react";
import BellButton from "../kokonutui/bell-button";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import {
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverHeader,
  PopoverTitle,
  PopoverTrigger,
} from "../ui/popover";
import { useI18n } from "@/hooks/useI18n";
import { cn } from "@/lib/utils";

interface NotificationCenterProps {
  updateVersion?: string;
  pendingCount: number;
  reminder?: { kind: "break" | "stopwatch"; label: string };
  onOpenUpdates: () => void;
  onOpenReview: () => void;
  onOpenReminder: (kind: "break" | "stopwatch") => void;
  onOpenSettings: () => void;
}

interface NotificationItem {
  id: string;
  title: string;
  description: string;
  action: () => void;
}

export function NotificationCenter({
  updateVersion,
  pendingCount,
  reminder,
  onOpenUpdates,
  onOpenReview,
  onOpenReminder,
  onOpenSettings,
}: NotificationCenterProps) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const [read, setRead] = useState<Set<string>>(() => new Set());
  const items = useMemo<NotificationItem[]>(() => {
    const next: NotificationItem[] = [];
    if (updateVersion) {
      next.push({
        id: `update:${updateVersion}`,
        title: t("notifications.updateAvailable"),
        description: t("notifications.version", { version: updateVersion }),
        action: onOpenUpdates,
      });
    }
    if (pendingCount > 0) {
      next.push({
        id: `review:${pendingCount}`,
        title: t("notifications.pendingReview"),
        description: t("notifications.pendingCount", { count: pendingCount }),
        action: onOpenReview,
      });
    }
    if (reminder) {
      next.push({
        id: `reminder:${reminder.kind}:${reminder.label}`,
        title:
          reminder.kind === "break"
            ? t("notifications.breakReminder")
            : t("notifications.stopwatchReminder"),
        description: reminder.label,
        action: () => onOpenReminder(reminder.kind),
      });
    }
    return next;
  }, [onOpenReminder, onOpenReview, onOpenUpdates, pendingCount, reminder, t, updateVersion]);
  const unreadCount = items.filter((item) => !read.has(item.id)).length;

  const run = (action: () => void): void => {
    setOpen(false);
    action();
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <span className="relative inline-flex">
        <PopoverTrigger
          render={
            <BellButton
              variant="icon"
              subscribed={open || unreadCount > 0}
              count={unreadCount}
              label={t("common.notifications")}
            />
          }
        />
        {unreadCount > 0 && (
          <span
            aria-hidden="true"
            className="pointer-events-none absolute -top-1 -right-1 flex min-w-4 items-center justify-center rounded-full bg-accent px-1 text-[10px] font-semibold leading-4 text-accent-fg"
          >
            {unreadCount > 9 ? "9+" : unreadCount}
          </span>
        )}
      </span>
      <PopoverContent align="end" sideOffset={8} className="w-88 border-line bg-panel p-0 text-fg">
        <PopoverHeader className="border-line border-b px-4 py-3">
          <div className="flex items-center justify-between gap-3">
            <PopoverTitle className="flex items-center gap-2 text-fg-strong">
              <BellRingIcon aria-hidden="true" />
              {t("notifications.title")}
            </PopoverTitle>
            {unreadCount > 0 && <Badge variant="secondary">{unreadCount}</Badge>}
          </div>
          <PopoverDescription className="text-fg-muted">
            {t("notifications.description")}
          </PopoverDescription>
        </PopoverHeader>

        <div className="flex max-h-80 flex-col gap-1 overflow-y-auto p-2">
          {items.length === 0 ? (
            <div className="flex flex-col items-center gap-2 px-4 py-8 text-center">
              <CheckCheckIcon aria-hidden="true" className="text-accent" />
              <p className="text-sm font-medium text-fg-strong">{t("notifications.empty")}</p>
              <p className="text-xs text-fg-muted">{t("notifications.emptyDescription")}</p>
            </div>
          ) : (
            items.map((item) => {
              const isRead = read.has(item.id);
              return (
                <button
                  className={cn(
                    "flex flex-col gap-1 rounded-lg px-3 py-2 text-left transition-colors hover:bg-surface-strong",
                    isRead && "opacity-60",
                  )}
                  key={item.id}
                  onClick={() => run(item.action)}
                  type="button"
                >
                  <span className="flex items-center justify-between gap-3 text-xs font-semibold text-fg-strong">
                    {item.title}
                    {!isRead && <span className="size-1.5 rounded-full bg-accent" />}
                  </span>
                  <span className="text-xs text-fg-muted">{item.description}</span>
                </button>
              );
            })
          )}
        </div>

        <div className="flex items-center justify-between gap-2 border-line border-t px-3 py-2">
          <Button
            disabled={unreadCount === 0}
            onClick={() => setRead(new Set(items.map((item) => item.id)))}
            size="sm"
            variant="ghost"
          >
            <CheckCheckIcon data-icon="inline-start" />
            {t("notifications.markAllRead")}
          </Button>
          <Button onClick={() => run(onOpenSettings)} size="sm" variant="ghost">
            <Settings2Icon data-icon="inline-start" />
            {t("notifications.settings")}
          </Button>
        </div>
      </PopoverContent>
    </Popover>
  );
}
