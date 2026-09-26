from __future__ import annotations

import re


CATEGORY_RULES: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("build-buy", ("build buy", "buildbuy", "build mode", "buy mode", "builder")),
    ("cas", ("create a sim", "create-a-sim", "cas", "sim creation")),
    ("relationships", ("relationship", "romance", "dating", "pregnancy", "family")),
    ("story-progression", ("story progression", "population", "neighborhood stories")),
    ("careers", ("career", "job", "workplace")),
    ("traits", ("trait", "aspiration")),
    ("interface", ("ui", "interface", "hud")),
    ("gameplay", ("gameplay", "autonomy", "interaction", "simulation")),
    ("utilities", ("utility", "tool", "debug", "exception", "diagnostic")),
)

FEATURE_RULES: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("cheats", ("cheat", "command center", "command centre")),
    ("relationship-management", ("relationship", "romance", "dating")),
    ("pregnancy-family", ("pregnancy", "family", "fertility")),
    ("population-management", ("population", "story progression", "neighborhood stories")),
    ("build-tools", ("build buy", "buildbuy", "object placement", "builder")),
    ("ui-controls", ("ui", "interface", "hud")),
    ("diagnostics", ("exception", "diagnostic", "error report")),
    ("career-gameplay", ("career", "job", "workplace")),
    ("trait-gameplay", ("trait", "aspiration")),
    ("autonomy", ("autonomy", "interaction tuning")),
)


def infer_taxonomy(
    *,
    name: str,
    summary: str | None,
    slug: str,
) -> tuple[list[str], list[str]]:
    haystack = _normalize_text(" ".join([name, summary or "", slug]))

    categories = [
        label
        for label, keywords in CATEGORY_RULES
        if any(_contains_phrase(haystack, keyword) for keyword in keywords)
    ]
    features = [
        label
        for label, keywords in FEATURE_RULES
        if any(_contains_phrase(haystack, keyword) for keyword in keywords)
    ]

    return sorted(set(categories)), sorted(set(features))


def _normalize_text(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", " ", value.casefold()).strip()


def _contains_phrase(haystack: str, phrase: str) -> bool:
    normalized = _normalize_text(phrase)
    return f" {normalized} " in f" {haystack} "
