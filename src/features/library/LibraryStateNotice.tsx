import { useI18n } from "../../i18n/i18n";
export type LibraryRegistryState = "ready" | "offline" | "partial" | "failure";

type LibraryStateNoticeProps = {
  state: LibraryRegistryState;
};

const copy: Record<Exclude<LibraryRegistryState, "ready">, { title: string; detail: string }> = {
  offline: {
    title: "Registry offline",
    detail: "Local files, duplicates and cached identities are still available. Online provenance may be stale."
  },
  partial: {
    title: "Partial registry results",
    detail: "Some source adapters are unavailable. Existing local evidence remains visible while missing source data is marked."
  },
  failure: {
    title: "Registry request failed",
    detail: "The local library is intact. Retry the registry lookup when connectivity or the upstream source recovers."
  }
};

export function LibraryStateNotice({ state }: LibraryStateNoticeProps) {
  const { t } = useI18n();
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
        <strong>{t(message.title as keyof typeof import("../../i18n/i18n"))}</strong>
        <p>{message.detail}</p>
      </div>
      {state === "failure" && <button className="button button--secondary">Retry</button>}
    </section>
  );
}
