# versus

Pairwise ranking of any list of items: people compare two at a time, and a
Bradley–Terry model with Fisher-optimal pair selection ranks them all. Each
site is described by its `site.toml` (see `server/src/config.rs`); the 48 Hour
Film Project sites are the first ones.

Cargo workspace: **`shared/`** (types), **`server/`** (actix-web API + SSE), **`client/`** (Leptos WASM SPA).

## Sites

A site is a folder holding `site.toml`, `data.csv`, `db.json`, `backups/`,
`banned.txt` and, when it is editable, `edits.jsonl` and `uploads/`. The server
runs with that folder as its working directory. On rtx-home every site is a
container of the `geiger` account; `~/git/geiger-server` deploys them
(`./deploy versus` rebuilds and restarts every site).

### Wiki

With `editable = true`, anyone may edit an item's fields, upload its poster
and propose new items. `data.csv` stays the original; every change is a
revision appended to `edits.jsonl`, and the catalog is the fold of that
journal over the CSV (`server/src/catalog.rs`). Edits apply at once and can be
reverted from the item's history; a proposed item enters the ranking once
someone other than its author approves it. Writes are rate-limited per user
and per address, and the authors in `banned.txt` count for nothing.
`/api/films.csv` gives the current catalog in the shape of `data.csv`.

`lang = "fr"` or `"en"` chooses the interface's language (`client/src/i18n.rs`);
the copy a site sets in its `site.toml` takes precedence.

## Development

```bash
# Terminal 1: server on :4848, in a site folder (this repo's root is one)
RUST_LOG=info cargo run -p versus-server

# Terminal 2: WASM client on :8080 (proxies /api/ and /uploads/ to :4848)
cd client && trunk serve
```

## API

| Endpoint | Method | Description |
|---|---|---|
| `/health` | GET | Health check (try-locks internal mutexes) |
| `/api/config` | GET | The site's client-facing configuration |
| `/api/films` | GET | List the published items |
| `/api/films` | POST | Propose an item |
| `/api/films.csv` | GET | The published items as `data.csv` |
| `/api/films/{id}` | POST | Edit an item |
| `/api/films/{id}/revert` | POST | Restore an item as a revision left it |
| `/api/films/{id}/review` | POST | Approve or reject a proposed item |
| `/api/films/{id}/history` | GET | An item's revisions |
| `/api/proposals` | GET | Proposed items awaiting review |
| `/api/uploads` | POST | Upload a poster (JPEG) |
| `/uploads/{name}` | GET | An uploaded poster |
| `/api/selection` | POST | Set user's seen items |
| `/api/pair` | GET | Next pair to compare (D-optimal) |
| `/api/vote` | POST | Submit a vote |
| `/api/unvote` | POST | Undo a vote |
| `/api/reset-votes` | POST | Clear all user votes |
| `/api/vote/stream` | GET | SSE stream of votes and catalog changes |
| `/api/leaderboard` | GET | Rankings (JSON) |
| `/api/leaderboard.csv` | GET | Rankings (CSV) |
| `/api/stats` | GET | Aggregate statistics |
| `/api/user-contributions` | GET | Per-user vote counts |
| `/api/user-matrix` | GET | User's pairwise vote matrix |
| `/api/global-matrix` | GET | Global win/loss matrix |
