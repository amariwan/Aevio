"use client";

import { Button } from "@/components/ui/button";
import { useBreaks } from "@/hooks/useBreaks";
import { useI18n } from "@/hooks/useI18n";
import * as api from "@/lib/api";
import { formatCountdown } from "@/lib/breaks";

export default function BreakScreen() {
  const { state, now } = useBreaks();
  const { t } = useI18n();
  const current = state.current;
  const remainingMs = current ? current.startedAt + current.plannedMs - now : 0;

  return (
    <main className="flex min-h-screen min-w-full items-center justify-center bg-scrim/65 px-6 text-fg backdrop-blur-xl">
      <section className="w-full max-w-[360px] rounded-xl border border-line-strong bg-panel p-5 text-center shadow-2xl">
        <div className="border-b border-line pb-4 text-xs font-semibold uppercase tracking-[0.2em] text-accent">
          {t("break.screenBrand")}
        </div>
        <h1 className="mt-6 text-3xl font-semibold text-fg-strong">{t("break.screenTitle")}</h1>
        <p className="mt-4 text-sm leading-5 text-fg-muted">{t("break.screenDescription")}</p>
        <p className="mt-8 font-mono text-4xl text-accent tabular-nums">
          {formatCountdown(remainingMs)}
        </p>
        <div className="mt-10 flex flex-col gap-2">
          <Button variant="secondary" onClick={() => void api.extendBreak()}>
            {t("break.extendFive")}
          </Button>
          <Button onClick={() => void api.endBreak()}>{t("break.end")}</Button>
        </div>
      </section>
    </main>
  );
}
