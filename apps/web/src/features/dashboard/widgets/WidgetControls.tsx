import { Check, ChevronDown, ChevronUp, RotateCcw, Settings2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/hooks/useI18n";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { DASHBOARD_WIDGET_REGISTRY } from "./registry";
import type { DashboardWidgetId, DashboardWidgetLayout } from "./types";

export function WidgetControls({
  layout,
  onToggle,
  onMove,
  onReset,
}: {
  layout: DashboardWidgetLayout;
  onToggle: (id: DashboardWidgetId) => void;
  onMove: (id: DashboardWidgetId, direction: -1 | 1) => void;
  onReset: () => void;
}) {
  const { t } = useI18n();
  return (
    <Dialog>
      <DialogTrigger
        render={<Button type="button" size="sm" variant="outline" className="min-h-11" />}
      >
        <Settings2 aria-hidden="true" />
        Customize layout
      </DialogTrigger>
      <DialogContent className="max-h-[min(80vh,42rem)] max-w-xl overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t("dashboard.customize")}</DialogTitle>
          <DialogDescription>
            Show, hide, and reorder widgets to match your daily workflow.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-2 sm:grid-cols-2">
          {layout.order.map((id, index) => {
            const widget = DASHBOARD_WIDGET_REGISTRY[id];
            const visible = !layout.hidden.includes(id);
            return (
              <div
                key={id}
                className="flex items-center gap-2 rounded-lg border border-line bg-canvas px-3 py-2"
              >
                <Button
                  type="button"
                  size="icon-sm"
                  variant={visible ? "secondary" : "ghost"}
                  aria-label={`${visible ? "Hide" : "Show"} ${widget.label}`}
                  aria-pressed={visible}
                  onClick={() => onToggle(id)}
                >
                  <Check aria-hidden="true" className={visible ? "" : "invisible"} />
                </Button>
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium text-fg-strong">{widget.label}</p>
                  <p className="truncate text-xs text-fg-muted">{widget.description}</p>
                </div>
                <div className="flex shrink-0">
                  <Button
                    type="button"
                    size="icon-sm"
                    variant="ghost"
                    aria-label={`Move ${widget.label} up`}
                    disabled={index === 0}
                    onClick={() => onMove(id, -1)}
                  >
                    <ChevronUp aria-hidden="true" />
                  </Button>
                  <Button
                    type="button"
                    size="icon-sm"
                    variant="ghost"
                    aria-label={`Move ${widget.label} down`}
                    disabled={index === layout.order.length - 1}
                    onClick={() => onMove(id, 1)}
                  >
                    <ChevronDown aria-hidden="true" />
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
        <DialogFooter>
          <Button type="button" variant="ghost" onClick={onReset}>
            <RotateCcw aria-hidden="true" />
            Reset
          </Button>
          <DialogClose render={<Button type="button" />}>{t("common.done")}</DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
