from __future__ import annotations

import os
from dataclasses import dataclass
from functools import lru_cache


@dataclass(frozen=True, slots=True)
class Settings:
    database_url: str
    curseforge_api_key: str | None = None
    curseforge_base_url: str = "https://api.curseforge.com"
    api_title: str = "Sims Mod Health Registry API"
    api_version: str = "0.1.0"


@lru_cache
def get_settings() -> Settings:
    return Settings(
        database_url=os.getenv(
            "REGISTRY_DATABASE_URL",
            "postgresql+psycopg://postgres:postgres@localhost:5432/sims_mod_health",
        ),
        curseforge_api_key=os.getenv("CURSEFORGE_API_KEY"),
        curseforge_base_url=os.getenv(
            "CURSEFORGE_BASE_URL",
            "https://api.curseforge.com",
        ),
    )
