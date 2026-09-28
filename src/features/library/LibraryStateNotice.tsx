import { useI18n } from "../../i18n/I18nProvider";
export type LibraryRegistryState = "ready" | "offline" | "partial" | "failure";

type LibraryStateNoticeProps = {
  state: LibraryRegistryState;
};

export function LibraryStateNotice({ state }: LibraryStateNoticeProps) {
  const { t } = useI18n();
  if (state === "ready") {
    return null;
  }

  const message =
    state === "offline"
      ? { title: t("library.registryOffline"), detail: t("library.registryOfflineCopy") }
      : state === "partial"
        ? { title: t("library.registryPartial"), detail: t("library.registryPartialCopy") }
        : { title: t("library.registryFailure"), detail: t("library.registryFailureCopy") };

  return (
    <section className={"library-state library-state--" + state} role="status">
      <span className="library-state__symbol" aria-hidden="true">
        {state === "failure" ? "×" : "!"}
      </span>
      <div>
        <strong>{message.title}</strong>
        <p>{message.detail}</p>
      </div>
      {state === "failure" && <button className="button button--secondary">{t("library.retry")}</button>}
    </section>
  );
}
