import { useI18n } from "../../i18n/i18n";

export function LanguageSwitcher() {
  const { locale, setLocale, t } = useI18n();

  return (
    <div className="language-switcher" aria-label={t("Application language")}>
      <span>{t("Application language")}</span>
      <div>
        <button
          type="button"
          aria-pressed={locale === "en"}
          aria-label={t("English")}
          onClick={() => setLocale("en")}
        >
          EN
        </button>
        <button
          type="button"
          aria-pressed={locale === "fr"}
          aria-label={t("French")}
          onClick={() => setLocale("fr")}
        >
          FR
        </button>
      </div>
    </div>
  );
}
