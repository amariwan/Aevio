import { useCallback, useMemo, useState } from "react";
import { DASHBOARD_WIDGET_IDS, DASHBOARD_WIDGET_REGISTRY } from "./registry";
import type { DashboardWidgetId, DashboardWidgetLayout } from "./types";

const STORAGE_KEY = "aevio.dashboard.widget-layout.v1";

function defaultLayout(): DashboardWidgetLayout {
  return { order: [...DASHBOARD_WIDGET_IDS], hidden: [] };
}

function readLayout(): DashboardWidgetLayout {
  if (typeof window === "undefined") return defaultLayout();
  try {
    const parsed = JSON.parse(
      window.localStorage.getItem(STORAGE_KEY) ?? "null",
    ) as Partial<DashboardWidgetLayout> | null;
    const order = Array.isArray(parsed?.order)
      ? parsed.order.filter((id): id is DashboardWidgetId =>
          DASHBOARD_WIDGET_IDS.includes(id as DashboardWidgetId),
        )
      : [];
    const missing = DASHBOARD_WIDGET_IDS.filter((id) => !order.includes(id));
    const hidden = Array.isArray(parsed?.hidden)
      ? parsed.hidden.filter((id): id is DashboardWidgetId =>
          DASHBOARD_WIDGET_IDS.includes(id as DashboardWidgetId),
        )
      : [];
    return { order: [...order, ...missing], hidden };
  } catch {
    return defaultLayout();
  }
}

export function useWidgetLayout() {
  const [layout, setLayout] = useState<DashboardWidgetLayout>(readLayout);

  const persist = useCallback((next: DashboardWidgetLayout): void => {
    setLayout(next);
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
    } catch {
      // Layout remains usable for this session when storage is unavailable.
    }
  }, []);

  const toggle = useCallback(
    (id: DashboardWidgetId): void => {
      persist({
        ...layout,
        hidden: layout.hidden.includes(id)
          ? layout.hidden.filter((item) => item !== id)
          : [...layout.hidden, id],
      });
    },
    [layout, persist],
  );

  const move = useCallback(
    (id: DashboardWidgetId, direction: -1 | 1): void => {
      const index = layout.order.indexOf(id);
      const nextIndex = index + direction;
      if (index < 0 || nextIndex < 0 || nextIndex >= layout.order.length) return;
      const order = [...layout.order];
      const [item] = order.splice(index, 1);
      if (item) order.splice(nextIndex, 0, item);
      persist({ ...layout, order });
    },
    [layout, persist],
  );

  const reset = useCallback((): void => persist(defaultLayout()), [persist]);

  const visibleIds = useMemo(
    () =>
      layout.order.filter(
        (id) => !layout.hidden.includes(id) && DASHBOARD_WIDGET_REGISTRY[id].defaultVisible,
      ),
    [layout],
  );

  return { layout, visibleIds, toggle, move, reset };
}
