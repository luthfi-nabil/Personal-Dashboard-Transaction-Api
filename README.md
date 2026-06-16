# Transaction API

Personal finance backend for the **personal-dashboard** suite. Built with
[Actix-Web](https://actix.rs/) (Rust) and MySQL/MariaDB, it manages a user's
financial sources (wallets/accounts), earnings, spendings, debts, and app
settings, and is consumed by the [Flutter dashboard](../personal_dashboard_flutter)
and the [Telegram bot](../telegram-bot).

## Purpose

- Track money **sources** (bank accounts, e-wallets, cash) and their running balances.
- Record **earnings** and **spendings**, each tagged with a category and a source.
- Manage **earning/spending categories** per user.
- Track **debts** and their settlement status.
- Provide a small **app settings** store (e.g. category IDs used for transfers,
  recounts, and debt bookkeeping — consumed by `post_earning_api_v2` to special-case
  those categories).
- Expose a **Flutter sync** endpoint (`/api/flutter/sync*`) used by the mobile app's
  offline-cache sync.

Every user-scoped endpoint lives under `/api/user/...` and requires an
`Authorization: Bearer <jwt>` header. The API validates the token and uses its
signed `sub` claim as `created_by`.

## Tech stack

- **Actix-Web 4** — HTTP server/routing
- **MySQL/MariaDB** (via the `mysql` crate) — primary datastore
- **actix-cors** — CORS (required for the Flutter web app / browser preflight requests)
- **tracing** + **tracing-appender** — daily-rotating request logs under `logs/`
- **jsonwebtoken** / **bcrypt** — used by the Flutter sync JWT flow

## Project structure

```
src/
  main.rs               # entrypoint: logging, CORS, routing setup
  routes/main_route.rs  # route table
  handlers/             # request handlers (sources, earnings, spendings, debts, settings, sync, swagger)
  repository/           # SQL queries per domain
  models/               # request/response structs
  route_middleware/      # CreatedBy extraction, JSON error wrapping
  helper/                # DB connection, response codes
swagger.yaml             # OpenAPI 3 spec, served at /docs in development
```

## Prerequisites

- Rust toolchain (edition 2024 — see `Cargo.toml`)
- A running MySQL/MariaDB server
- The `transaction_db` schema, imported from [`../transaction_db.sql`](../transaction_db.sql)

```bash
mysql -u root -p < ../transaction_db.sql
```

## Configuration

Configuration is read from a `.env` file in this directory (via `dotenv`):

| Variable    | Default        | Description |
|-------------|----------------|-------------|
| `HOST`      | `127.0.0.1`    | Bind address. Use `0.0.0.0` to accept connections from other devices (e.g. a phone running the Flutter app on the same LAN). |
| `PORT`      | `8080`         | Bind port (project default is `3000`). |
| `APP_ENV`   | `production`   | Set to `development` to enable Swagger UI at `/docs` and `/docs/openapi.yaml`. |
| `DB_HOST`   | `127.0.0.1`    | MySQL host. |
| `DB_PORT`   | `3306`         | MySQL port. |
| `DB_USER`   | `root`         | MySQL user. |
| `DB_PASS`   | `123456`       | MySQL password. |
| `DB_NAME`   | `transaction`  | MySQL database name (matches `transaction_db.sql`). |
| `RUST_BACKTRACE` | -         | Set to `1` for full panic backtraces during development. |

Example `.env`:

```env
RUST_BACKTRACE=1
HOST=0.0.0.0
PORT=3000
APP_ENV=development
DB_HOST=127.0.0.1
DB_PORT=3306
DB_USER=root
DB_PASS=123456
DB_NAME=transaction
```

## Running locally

```bash
cargo run
```

The server prints its bound address on startup:

```
Server running at http://0.0.0.0:3000
Swagger UI   →  http://0.0.0.0:3000/docs
OpenAPI spec →  http://0.0.0.0:3000/docs/openapi.yaml
```

## API documentation

With `APP_ENV=development`, browse to `/docs` for the Swagger UI, backed by
[`swagger.yaml`](swagger.yaml). Key endpoint groups:

- `GET /api/settings`, `GET /api/user/settings` — app settings
- `GET/POST/DELETE /api/user/source[-balance|/...]` — sources & balances
- `GET/POST /api/user/earnings`, `GET/POST/DELETE .../earning-categories[/{category}]`
- `GET/POST /api/user/spendings`, `GET/POST/DELETE .../spending-categories[/{category}]`
- `GET/POST/PUT /api/user/debt`, `.../debt-status`
- `GET /api/flutter/sync`, `POST /api/flutter/sync/push` — offline-cache sync for the Flutter app

## Logging

Requests are logged to daily-rotating files under [`logs/`](logs)
(`actix.log.YYYY-MM-DD`), recording timestamp, client IP, method + path,
status code, response size, duration, and User-Agent.

## CORS

`Cors::permissive()` is applied as the outermost middleware so that browser
preflight (`OPTIONS`) requests from the Flutter web build succeed. Without it,
preflight requests 404 and the browser blocks the real request.

## Deployment

A [`Dockerfile`](Dockerfile) is provided for the build stage (`cargo build`);
the runtime stage is left as a template (commented out) for you to complete —
e.g. copy `target/debug/transaction-api` (or `target/release/...` with
`cargo build --release`) into a slim runtime image along with a `.env` file.

For local network access (e.g. testing from a phone), set `HOST=0.0.0.0` and
point the Flutter app's "Transaction API base URL" (Settings screen) at this
machine's LAN IP, e.g. `http://192.168.1.x:3000`. Ensure your firewall allows
inbound connections to the chosen port.
