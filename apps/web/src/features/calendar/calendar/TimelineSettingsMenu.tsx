import { MenuIcon } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useI18n } from "@/hooks/useI18n";

import { columnLabel, type CalendarLayout } from "./columns";
type ZoomLevel = string;
const ZOOM_LEVELS: readonly ZoomLevel[] = ["15m", "30m", "1h", "2h", "4h", "8h", "1d"];

interface TimelineSettingsMenuProps {
  layout: CalendarLayout;
  onLayoutChange: (layout: CalendarLayout) => void;
  onZoomLevel: (level: ZoomLevel) => void;
  onZoomIn: () => void;
  onZoomOut: () => void;
}

export function TimelineSettingsMenu({
  layout,
  onLayoutChange,
  onZoomLevel,
  onZoomIn,
  onZoomOut,
}: TimelineSettingsMenuProps) {
  const { t } = useI18n();
  const columns = layout.visible;
  const onColumnsChange = (next: typeof columns): void =>
    onLayoutChange({ ...layout, visible: next });
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        className="flex size-8 items-center justify-center rounded-md text-fg-faint transition-colors hover:bg-surface hover:text-fg"
        aria-label={t("calendar.timelineMenu")}
      >
        <MenuIcon className="size-4" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-72 p-1.5">
        <DropdownMenuGroup>
          <DropdownMenuLabel>{t("calendar.zoom")}</DropdownMenuLabel>
          <div className="flex items-center gap-1 px-1 pb-1">
            {ZOOM_LEVELS.map((level) => (
              <button
                key={level}
                type="button"
                onClick={() => onZoomLevel(level)}
                className={`rounded px-1.5 py-1 text-[11px] hover:bg-surface ${level === "15m" ? "bg-accent/15 font-semibold text-accent" : "text-fg-muted"}`}
              >
                {level}
              </button>
            ))}
          </div>
          <DropdownMenuItem onClick={() => onZoomLevel("15m")}>
            {t("calendar.changeDefaultRange")}
          </DropdownMenuItem>
          <DropdownMenuItem onClick={onZoomIn}>{t("calendar.zoomIn")}</DropdownMenuItem>
          <DropdownMenuItem onClick={onZoomOut}>{t("calendar.zoomOut")}</DropdownMenuItem>
          <DropdownMenuItem>{t("calendar.switchToUnifiedView")}</DropdownMenuItem>
          <DropdownMenuSub>
            <DropdownMenuSubTrigger>{t("calendar.toggleColumns")}</DropdownMenuSubTrigger>
            <DropdownMenuSubContent className="min-w-48">
              <DropdownMenuGroup>
                {layout.order.map((column) => (
                  <DropdownMenuCheckboxItem
                    key={column}
                    checked={columns[column]}
                    onCheckedChange={(checked) =>
                      onColumnsChange({ ...columns, [column]: checked === true })
                    }
                  >
                    {columnLabel(column, t)}
                  </DropdownMenuCheckboxItem>
                ))}
              </DropdownMenuGroup>
            </DropdownMenuSubContent>
          </DropdownMenuSub>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
