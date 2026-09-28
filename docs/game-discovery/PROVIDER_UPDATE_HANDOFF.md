# Official Provider Update Handoff

## Goal

Sims Mod Health may identify that the installed game or one of its locally present content packs requires attention, but it does not become a game launcher, entitlement manager or arbitrary process runner.

The production update path is:

```text
Health finding
  -> official provider handoff
  -> persistent AwaitingRescan
  -> local game/DLC refresh
  -> incremental Mods rescan
  -> Game/DLC health verification
  -> Mod health reevaluation
```

## Provider strategies

The native Rust boundary owns provider resolution.

### EA app

The adapter searches only known EA Desktop locations and EA Desktop registry installation locations, then validates that the canonical executable filename is exactly:

```text
EADesktop.exe
```

### Steam

The adapter searches the Steam client registry location and standard Steam installation locations, then validates that the canonical executable filename is exactly:

```text
steam.exe
```

### Unknown provider

No process is launched. The session stays `action_required` with manual instructions. After the user updates through their normal provider, the same session can enter local verification.

## Security boundary

Provider launch is implemented inside the privileged Rust command boundary with `std::process::Command`.

The WebView does **not** receive:

- `shell:open`;
- broad process permissions;
- an arbitrary executable path parameter;
- arbitrary command-line arguments;
- provider credentials.

The executable path is resolved by the adapter and is not supplied by UI input.

## State machine

Sessions are persisted in SQLite:

```text
detected
  -> action_required
     -> provider_opened
        -> awaiting_rescan
     -> awaiting_rescan        (manual/unknown provider)
  -> failed

awaiting_rescan
  -> verified
  -> still_outdated
  -> unknown
  -> failed
```

Terminal states do not transition further. A retry creates a new session, preserving the previous evidence trail.

Tables:

- `provider_update_sessions`
- `provider_update_events`

Because `awaiting_rescan` is persisted rather than held in memory, closing and reopening Sims Mod Health does not cause the app to assume success or forget that verification is still required.

## Native commands

```text
get_provider_update_capability()
start_game_content_provider_update(targetKind, targetId)
get_game_content_provider_update_session(sessionId)
get_pending_game_content_provider_update_session()
verify_game_content_provider_update(sessionId)
```

The pending-session command is the restart recovery entry point for the UI.

## Verification workflow

`verify_game_content_provider_update(sessionId)` performs:

1. transition to or retain `awaiting_rescan`;
2. rediscover the game program installation;
3. persist fresh Game/DLC inventory;
4. read the new authoritative program build;
5. run an incremental Mods scan when a local Mods installation is known;
6. synchronize the mod-health installation patch to the freshly observed program build;
7. run Game/DLC manifest health;
8. run the existing Registry-backed Mod health pipeline;
9. transition the provider session according to local evidence.

### Final state mapping

- target health `current` -> `verified`;
- `update_available` or `game_update_required` -> `still_outdated`;
- stale, disputed, unknown or integrity-uncertain evidence -> `unknown`;
- rescan/orchestration failure -> `failed`.

Opening EA app or Steam never produces `verified` by itself.

## UI contract

No new navigation destination is introduced.

The intended compact interaction for #76 is:

```text
Health finding
[Open EA app to update]
          |
          v
Waiting for verification
[Verify update]
```

Settings may show the detected provider/capability, but provider management is not a separate product area.

## Non-goals

This workflow does not:

- download game/DLC payloads;
- bypass provider ownership checks;
- unlock DLC;
- collect or replay credentials;
- launch user-selected executables;
- silently conclude that an update succeeded because a provider process started.
