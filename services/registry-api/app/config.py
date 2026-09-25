from __future__ import annotations

import os
from dataclasses import dataclass
from functools import lru_cache


@dataclass(frozen=True, slots=True)
class Settings:
    database_url: str
    api_title: str = "Sims Mod Health Registry API"
    api_version: str = "0.1.0"


@lru_cache
def get_settings() -> Settings:
    return Settings(
        database_url=os.getenv(
            "REGISTRY_DATABASE_URL",
            "postgresql+psycopg://postgres:postgres@localhost:5432/sims_mod_health",
        )
    )
