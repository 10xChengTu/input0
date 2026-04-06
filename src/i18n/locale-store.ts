import { create } from "zustand";
import type { Locale, Translations } from "./types";
import { zh } from "./zh";
import { en } from "./en";

const translationsMap: Record<Locale, Translations> = { zh, en };

interface LocaleState {
  locale: Locale;
  t: Translations;
  setLocale: (locale: Locale) => void;
  toggleLocale: () => void;
}

function getInitialLocale(): Locale {
  if (typeof window === "undefined") return "zh";
  const stored = localStorage.getItem("input0-locale");
  if (stored === "zh" || stored === "en") return stored;
  return "zh";
}

export const useLocaleStore = create<LocaleState>((set) => {
  const initial = getInitialLocale();

  return {
    locale: initial,
    t: translationsMap[initial],
    setLocale: (locale) => {
      localStorage.setItem("input0-locale", locale);
      set({ locale, t: translationsMap[locale] });
    },
    toggleLocale: () => {
      set((state) => {
        const next: Locale = state.locale === "zh" ? "en" : "zh";
        localStorage.setItem("input0-locale", next);
        return { locale: next, t: translationsMap[next] };
      });
    },
  };
});
