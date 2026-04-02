# FilmRank

Pairwise film ranking for the 48 Hour Film Project. Bradley-Terry model with Fisher-optimal pair selection.

Cargo workspace: **`shared/`** (types), **`server/`** (actix-web API + SSE), **`client/`** (Leptos WASM SPA).

## Build & deploy

```bash
cargo build --release -p filmrank-server && trunk build --release --config client/Trunk.toml && systemctl --user restart 48hfp.service
```

## Development

```bash
# Terminal 1: server on :4848
RUST_LOG=info cargo run -p filmrank-server

# Terminal 2: WASM client on :8080 (proxies /api/ to :4848)
cd client && trunk serve
```

## API

| Endpoint | Method | Description |
|---|---|---|
| `/health` | GET | Health check (try-locks internal mutexes) |
| `/api/films` | GET | List all films |
| `/api/selection` | POST | Set user's seen films |
| `/api/pair` | GET | Next pair to compare (D-optimal) |
| `/api/vote` | POST | Submit a vote |
| `/api/unvote` | POST | Undo a vote |
| `/api/reset-votes` | POST | Clear all user votes |
| `/api/vote/stream` | GET | SSE stream of live votes |
| `/api/leaderboard` | GET | Rankings (JSON) |
| `/api/leaderboard.csv` | GET | Rankings (CSV) |
| `/api/stats` | GET | Aggregate statistics |
| `/api/user-contributions` | GET | Per-user vote counts |
| `/api/user-matrix` | GET | User's pairwise vote matrix |
| `/api/global-matrix` | GET | Global win/loss matrix |

## systemd

Socket activation on `:4848` (`48hfp.socket`), auto-restart on crash (`48hfp.service`), watchdog every 30s curls `/health` and restarts on failure (`48hfp-watchdog.timer`).

```bash
systemctl --user start 48hfp.socket          # start
systemctl --user restart 48hfp.service        # restart
journalctl --user -u 48hfp.service -f         # logs
```
