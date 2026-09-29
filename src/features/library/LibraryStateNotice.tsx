import { useI18n } from "../../i18n/i18n";
export type LibraryRegistryState = "ready" | "offline" | "partial" | "failure";

type LibraryStateNoticeProps = {
  state: LibraryRegistryState;
};

const copy: Record<Exclude<LibraryRegistryState, "ready">, { title: string; detail: string }> = {
  offline: {
    title: "Online details are temporarily unavailable",
    detail: "Your local files and duplicate checks still work. Mod names, compatibility and update information may be incomplete for now."
  },
  partial: {
    title: "Some online details could not be loaded",
    detail: "Your local files are still available. A few mod names, compatibility checks or update details may be missing."
  },
  failure: {
    title: "Online details could not be loaded",
    detail: "Your local library is safe. Try again later to refresh mod names, compatibility and update information."
  }
};

export function LibraryStateNotice({ state }: LibraryStateNoticeProps) {
  const { t, tx } = useI18n();
  if (state === "ready") {
    return null;
  }

  const message = copy[state];

  return (
    <section className={"library-state library-state--" + state} role="status">
      <span className="library-state__symbol" aria-hidden="true">
        {state === "failure" ? "×" : "!"}
      </span>
      <div>
        <strong>{tx(message.title)}</strong>
        <p>{tx(message.detail)}</p>
      </div>
      {state === "failure" && <button className="button button--secondary">{t("Retry")}</button>}
    </section>
  );
}
