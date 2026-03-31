# FilmRank

A pairwise film ranking app for the 48 Hour Film Project. Users select which films they've seen, then compare them head-to-head. Rankings are computed using the Bradley-Terry model with Fisher-optimal pair selection.

## Quick start

```bash
cargo build --release
./target/release/filmrank
```

The server starts on `http://localhost:4848`. Films are loaded from `data.csv`, votes are persisted to `db.json` with hourly backups in `backups/`.

Set `RUST_LOG=info` (or `debug`) to control log verbosity.

## API

| Endpoint | Method | Description |
|---|---|---|
| `/health` | GET | Health check (tries to acquire internal locks) |
| `/api/films` | GET | List all films |
| `/api/selection` | POST | Set which films a user has seen |
| `/api/pair` | GET | Get the next pair to compare |
| `/api/vote` | POST | Submit a vote |
| `/api/unvote` | POST | Undo a vote |
| `/api/vote/stream` | GET | SSE stream of live votes |
| `/api/leaderboard` | GET | Current rankings (JSON) |
| `/api/leaderboard.csv` | GET | Current rankings (CSV) |
| `/api/stats` | GET | Aggregate statistics |
| `/api/user-contributions` | GET | Per-user vote counts |
| `/api/user-matrix` | GET | A user's full vote matrix |
| `/api/global-matrix` | GET | Global win/loss matrix |

## systemd deployment

The server is managed by three systemd user units:

### `48hfp.socket` -- socket activation

```ini
[Socket]
ListenStream=0.0.0.0:4848
NoDelay=true
```

systemd opens port 4848 and passes the file descriptor to the service on first connection. This means the port is available immediately on boot, even before the binary starts. The server detects this via the `LISTEN_FDS` environment variable and uses fd 3 instead of binding the port itself.

### `48hfp.service` -- the application

```ini
[Service]
ExecStart=/home/mario/git/48hfp/target/release/filmrank
WorkingDirectory=/home/mario/git/48hfp
Restart=on-failure
Environment=RUST_LOG=info
TimeoutStopSec=3
```

- `Restart=on-failure` restarts the process if it crashes (non-zero exit).
- `RUST_LOG=info` enables request logging via `env_logger`.
- `TimeoutStopSec=3` gives the server 3 seconds to drain connections on stop.

### `48hfp-watchdog.timer` + `48hfp-watchdog.service` -- auto-restart on hang

A crash is caught by `Restart=on-failure`, but a deadlock or hang leaves the process alive and the port open -- systemd won't restart it. The watchdog handles this:

```ini
# 48hfp-watchdog.timer
[Timer]
OnBootSec=60
OnUnitActiveSec=30
```

```ini
# 48hfp-watchdog.service
[Service]
Type=oneshot
ExecStart=/bin/bash -c 'curl -sf --max-time 5 http://localhost:4848/health || systemctl --user restart 48hfp.service'
```

Every 30 seconds, the timer fires a one-shot service that curls `/health` with a 5-second timeout. The `/health` endpoint uses `try_lock()` on internal mutexes, so it returns 500 if the server is deadlocked rather than hanging. If the request fails or times out, the service is restarted.

### Managing the service

```bash
# Start/stop/restart
systemctl --user start 48hfp.socket
systemctl --user restart 48hfp.service

# Enable on login
systemctl --user enable 48hfp.socket 48hfp-watchdog.timer

# View logs
journalctl --user -u 48hfp.service -f

# Check watchdog status
systemctl --user status 48hfp-watchdog.timer
```
