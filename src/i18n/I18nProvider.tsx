import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode
} from "react";
import { messages, type Locale, type MessageKey } from "./messages";

export const STORAGE_KEY = "sims-mod-health.locale";

function detectLocale(): Locale {
  const saved = window.localStorage.getItem(STORAGE_KEY);
  if (saved === "fr" || saved === "en") return saved;
  return navigator.language.toLocaleLowerCase().startsWith("fr") ? "fr" : "en";
}

function interpolate(value: string, params?: Record<string, string | number>) {
  if (!params) return value;
  return value.replace(/\{([^}]+)\}/g, (_, key) => {
    const replacement = params[key];
    return replacement === undefined ? "{" + key + "}" : String(replacement);
  });
}

type I18nContextValue = {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (key: MessageKey, params?: Record<string, string | number>) => string;
};

const fallbackContext: I18nContextValue = {
  locale: "en",
  setLocale() {},
  t(key, params) {
    return interpolate(messages.en[key] ?? key, params);
  }
};

const I18nContext = createContext<I18nContextValue>(fallbackContext);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(detectLocale);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  const value = useMemo<I18nContextValue>(() => ({
    locale,
    setLocale(next) {
      window.localStorage.setItem(STORAGE_KEY, next);
      setLocaleState(next);
    },
    t(key, params) {
      const raw = messages[locale][key] ?? messages.en[key] ?? key;
      return interpolate(raw, params);
    }
  }), [locale]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  return useContext(I18nContext);
}
