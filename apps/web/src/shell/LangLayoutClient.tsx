"use client";

import { SettingsProvider } from "@/hooks/useSettings";
import { I18nProvider } from "@/hooks/useI18n";
import { AppShellProvider } from "@/shell/AppContext";

export default function LangLayoutClient({ children }: { children: React.ReactNode }) {
  return (
    <SettingsProvider>
      <I18nProvider>
        <AppShellProvider>{children}</AppShellProvider>
      </I18nProvider>
    </SettingsProvider>
  );
}
