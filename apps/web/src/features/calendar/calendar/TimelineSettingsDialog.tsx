import { Settings2Icon, XIcon } from "lucide-react";
import { type ReactNode, useState } from "react";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Checkbox } from "@/components/ui/checkbox";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { Slider } from "@/components/ui/slider";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { useI18n } from "@/hooks/useI18n";
import { useSettings } from "@/hooks/useSettings";

import { COLUMN_ICON, columnLabel, type CalendarLayout } from "./columns";
import { ChevronDownIcon, ChevronUpIcon, GripVerticalIcon } from "lucide-react";

interface TimelineSettingsDialogProps {
  layout: CalendarLayout;
  onLayoutChange: (layout: CalendarLayout) => void;
  compact?: boolean;
}

function sliderValue(value: number | readonly number[], fallback: number): number {
  return typeof value === "number" ? value : (value[0] ?? fallback);
}

function SettingRow({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-6 border-line border-b py-6 md:grid-cols-[minmax(0,0.88fr)_minmax(300px,1.12fr)]">
      <div>
        <h3 className="font-semibold text-[15px] text-fg-strong">{title}</h3>
        <p className="mt-1 max-w-sm text-[13px] text-fg-muted leading-5">{description}</p>
      </div>
      <div className="flex items-center">{children}</div>
    </div>
  );
}

export function TimelineSettingsDialog({
  layout,
  onLayoutChange,
  compact = false,
}: TimelineSettingsDialogProps) {
  const { t } = useI18n();
  const { settings, update } = useSettings();
  const [open, setOpen] = useState(false);
  const [entrySize, setEntrySize] = useState(15);
  const [dragging, setDragging] = useState<string | null>(null);

  const columns = layout.visible;
  const moveColumn = (column: (typeof layout.order)[number], direction: -1 | 1): void => {
    const index = layout.order.indexOf(column);
    const next = index + direction;
    if (next < 0 || next >= layout.order.length) return;
    const order = [...layout.order];
    [order[index], order[next]] = [order[next], order[index]];
    onLayoutChange({ ...layout, order });
  };
  const toggleColumn = (column: (typeof layout.order)[number], checked: boolean): void => {
    onLayoutChange({ ...layout, visible: { ...layout.visible, [column]: checked } });
  };
  const handleDrop = (target: (typeof layout.order)[number]): void => {
    if (!dragging || dragging === target) {
      setDragging(null);
      return;
    }
    const order = [...layout.order];
    const fromIndex = order.indexOf(dragging as (typeof layout.order)[number]);
    const toIndex = order.indexOf(target);
    if (fromIndex < 0 || toIndex < 0) {
      setDragging(null);
      return;
    }
    order.splice(fromIndex, 1);
    order.splice(toIndex, 0, dragging as (typeof layout.order)[number]);
    onLayoutChange({ ...layout, order });
    setDragging(null);
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className={
          compact
            ? "flex size-8 items-center justify-center rounded-md text-fg-faint transition-colors hover:bg-surface hover:text-fg"
            : "inline-flex h-7 items-center gap-1.5 rounded-md border border-line bg-panel px-2 font-medium text-[12px] text-fg-soft transition-colors hover:bg-surface hover:text-fg"
        }
        aria-label={t("calendar.timelineSettings")}
      >
        <Settings2Icon className="size-3.5" />
        {!compact && t("calendar.timelineSettings")}
      </button>
      <DialogContent
        showCloseButton={false}
        className="max-h-[min(92vh,900px)] max-w-3xl gap-0 overflow-hidden border-line bg-canvas p-0 shadow-2xl"
      >
        <div className="flex items-start justify-between border-line border-b px-6 py-5">
          <DialogHeader className="gap-1">
            <DialogTitle className="text-lg">{t("calendar.timelineSettings")}</DialogTitle>
            <DialogDescription>{t("calendar.timelineSettingsDescription")}</DialogDescription>
          </DialogHeader>
          <DialogClose className="rounded-md p-1 text-fg-muted hover:bg-surface hover:text-fg">
            <XIcon className="size-4" />
          </DialogClose>
        </div>

        <div className="overflow-y-auto px-6">
          <Tabs defaultValue="general" className="gap-0">
            <div className="border-line border-b py-4">
              <TabsList className="border border-line bg-panel">
                <TabsTrigger value="general">{t("calendar.general")}</TabsTrigger>
                <TabsTrigger value="prompts">{t("calendar.customPrompts")}</TabsTrigger>
              </TabsList>
            </div>

            <TabsContent value="general">
              <SettingRow
                title={t("calendar.tagSuggestionType")}
                description={t("calendar.tagSuggestionTypeDescription")}
              >
                <NativeSelect
                  value={settings.aiSuggest}
                  onChange={(event) =>
                    update({ aiSuggest: event.target.value as typeof settings.aiSuggest })
                  }
                  className="w-full"
                >
                  <NativeSelectOption value="category">
                    {t("calendar.autoDetect")}
                  </NativeSelectOption>
                  <NativeSelectOption value="categoryProject">
                    {t("calendar.aiPlusKeywords")}
                  </NativeSelectOption>
                </NativeSelect>
              </SettingRow>
              <SettingRow
                title={t("calendar.timeEntriesLanguage")}
                description={t("calendar.timeEntriesLanguageDescription")}
              >
                <NativeSelect
                  value={settings.language}
                  onChange={(event) =>
                    update({ language: event.target.value as typeof settings.language })
                  }
                  className="w-full"
                >
                  <NativeSelectOption value="system">
                    {t("calendar.systemDefault")}
                  </NativeSelectOption>
                  <NativeSelectOption value="en">{t("calendar.english")}</NativeSelectOption>
                  <NativeSelectOption value="de">{t("calendar.german")}</NativeSelectOption>
                </NativeSelect>
              </SettingRow>
              <SettingRow
                title={t("calendar.timeEntrySize")}
                description={t("calendar.timeEntrySizeDescription")}
              >
                <Slider
                  min={1}
                  max={30}
                  value={[entrySize]}
                  onValueChange={(value) => setEntrySize(sliderValue(value, 15))}
                  aria-label={t("calendar.timeEntrySizeAria")}
                />
              </SettingRow>
              <SettingRow
                title={t("calendar.autoAcceptTagSuggestions")}
                description={t("calendar.autoAcceptTagSuggestionsDescription")}
              >
                <div className="flex w-full items-center gap-3">
                  <Slider
                    min={50}
                    max={100}
                    value={[settings.autoAcceptPercent]}
                    onValueChange={(value) => update({ autoAcceptPercent: sliderValue(value, 95) })}
                  />
                  <span className="w-10 text-right text-sm text-accent">
                    {settings.autoAcceptPercent}%
                  </span>
                </div>
              </SettingRow>
              <SettingRow
                title={t("calendar.visibleColumns")}
                description={t("calendar.visibleColumnsDescription")}
              >
                <div className="w-full overflow-hidden rounded-lg border border-line bg-panel">
                  {layout.order.map((column, index) => {
                    const Icon = COLUMN_ICON[column];
                    return (
                      <div
                        key={column}
                        draggable
                        onDragStart={() => setDragging(column)}
                        onDragOver={(event) => event.preventDefault()}
                        onDrop={() => handleDrop(column)}
                        className="flex items-center gap-3 border-line border-b px-3 py-2.5 text-sm last:border-b-0 hover:bg-surface"
                      >
                        <GripVerticalIcon className="size-4 cursor-grab text-fg-faint" />
                        <Checkbox
                          checked={columns[column]}
                          onCheckedChange={(checked) => toggleColumn(column, checked === true)}
                        />
                        <Icon className="size-4 text-fg-muted" />
                        <span className="flex-1">{columnLabel(column, t)}</span>
                        <div className="flex items-center gap-1">
                          <button
                            type="button"
                            aria-label={t("calendar.moveColumnUp", {
                              column: columnLabel(column, t),
                            })}
                            disabled={index === 0}
                            onClick={() => moveColumn(column, -1)}
                            className="rounded p-1 text-fg-faint hover:bg-surface hover:text-fg disabled:opacity-30"
                          >
                            <ChevronUpIcon className="size-3.5" />
                          </button>
                          <button
                            type="button"
                            aria-label={t("calendar.moveColumnDown", {
                              column: columnLabel(column, t),
                            })}
                            disabled={index === layout.order.length - 1}
                            onClick={() => moveColumn(column, 1)}
                            className="rounded p-1 text-fg-faint hover:bg-surface hover:text-fg disabled:opacity-30"
                          >
                            <ChevronDownIcon className="size-3.5" />
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </SettingRow>
              <SettingRow
                title={t("calendar.minimumTimeEntrySize")}
                description={t("calendar.minimumTimeEntrySizeDescription")}
              >
                <NativeSelect className="w-full" defaultValue="8">
                  <NativeSelectOption value="8">{t("calendar.min8Recommended")}</NativeSelectOption>
                  <NativeSelectOption value="5">{t("calendar.min5")}</NativeSelectOption>
                  <NativeSelectOption value="15">{t("calendar.min15")}</NativeSelectOption>
                </NativeSelect>
              </SettingRow>
              <SettingRow
                title={t("calendar.timeEntryRounding")}
                description={t("calendar.timeEntryRoundingDescription")}
              >
                <NativeSelect className="w-full" defaultValue="off">
                  <NativeSelectOption value="off">{t("calendar.roundingOff")}</NativeSelectOption>
                  <NativeSelectOption value="5">{t("calendar.rounding5")}</NativeSelectOption>
                  <NativeSelectOption value="15">{t("calendar.rounding15")}</NativeSelectOption>
                </NativeSelect>
              </SettingRow>
            </TabsContent>
            <TabsContent value="prompts">
              <SettingRow
                title={t("calendar.customPromptsTitle")}
                description={t("calendar.customPromptsDescription")}
              >
                <Textarea
                  value={settings.aiCustomPrompt}
                  onChange={(event) => update({ aiCustomPrompt: event.target.value })}
                  placeholder={t("calendar.customPromptsPlaceholder")}
                />
              </SettingRow>
            </TabsContent>
          </Tabs>
        </div>
      </DialogContent>
    </Dialog>
  );
}
