import { createContext, type ReactNode, useContext, useEffect, useMemo } from "react";
import {
  localeFor,
  resolveLanguage,
  translate,
  type MessageKey,
  type ResolvedLanguage,
} from "@/lib/i18n";
import { useSettings } from "./useSettings";

interface I18nApi {
  language: ResolvedLanguage;
  locale: string;
  t: (key: MessageKey, values?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nApi>({
  language: "en",
  locale: "en-US",
  t: (key, values) => translate("en", key, values),
});

export function useI18n(): I18nApi {
  return useContext(I18nContext);
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const { settings } = useSettings();
  const language = resolveLanguage(
    settings.language,
    typeof navigator === "undefined" ? "en-US" : navigator.language,
  );
  const value = useMemo<I18nApi>(
    () => ({
      language,
      locale: localeFor(language),
      t: (key, values) => translate(language, key, values),
    }),
    [language],
  );

  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}
