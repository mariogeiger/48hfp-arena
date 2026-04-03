# Tech Stack

## Language

The entire project is written in **Rust** (Edition 2024) — both the backend and the frontend. No JavaScript is authored by hand; the frontend is compiled to **WebAssembly (WASM)**.

## Workspace Structure

Cargo workspace with three crates:

- `shared/` — types and data structures shared between server and client
- `server/` — HTTP API backend
- `client/` — WebAssembly frontend

## Backend

- **Actix-web** (v4) — HTTP server framework
- **Tokio** (v1, full features) — async runtime
- **serde / serde_json** — serialization and deserialization
- **Server-Sent Events (SSE)** — for live streaming updates to clients
- **File-based JSON persistence** — atomic writes with `.tmp` swap and backup rotation
- **env_logger / log** — structured logging via `RUST_LOG`
- **UUID** (v4) — user identification
- **rand** — random number generation

## Frontend

- **Leptos** (v0.8, CSR mode) — reactive component-based UI framework, signals model similar to SolidJS
- **wasm-bindgen / wasm-bindgen-futures** — Rust↔JavaScript interop and async support in WASM
- **web-sys** — low-level browser API bindings (DOM, Canvas, Fetch, EventSource, Storage, pointer/keyboard events, etc.)
- **js-sys** — JavaScript primitive bindings
- **console_error_panic_hook** — routes Rust panics to the browser console

## Build & Bundling

- **Cargo** — build system and package manager
- **Trunk** (v0.20+) — WASM bundler and dev server
  - Dev server on port 8080, proxies `/api/` to the backend on port 4848
  - Handles CSS bundling and static asset copying
- Release profile: `opt-level = "z"`, LTO, `codegen-units = 1`, `strip = true`, `panic = "abort"`

## Styling

Vanilla **CSS** with CSS custom properties for theming, responsive design using `100dvh` units, mobile-first viewport configuration.

## Deployment

- **systemd** user-level services with socket activation
- Watchdog timer for health checks and automatic restart on failure

## Code Quality

- **Pre-commit hooks** via `.pre-commit-config.yaml` — enforces `rustfmt` before every commit
