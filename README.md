# Transaction API

Personal finance backend for the **personal-dashboard** suite. Built with
[Actix-Web](https://actix.rs/) (Rust) and MySQL/MariaDB, it manages a user's
financial sources (wallets/accounts), earnings, spendings, and debts, and is
consumed by the [Flutter dashboard](../personal_dashboard_flutter) and the
[Telegram bot](../telegram-bot).

> **App settings moved (2026-07-27).** `app_settings` now lives in `login_db`
> and is served by [`login-api`](../login-api) at `/api/user/settings`. This
> service reads the global transfer/recount/debt category wiring from
> `GET {LOGIN_API_BASE}/api/settings` (see `src/helper/settings_client.rs`).

## Purpose

- Track money **sources** (bank accounts, e-wallets, cash) and their running balances.
- Record **earnings** and **spendings**, each tagged with a category and a source.
- Store an optional **line-item breakdown** per spending (`spending_detail`),
  filled in manually or from the Flutter app's receipt scanner.
- Manage **earning/spending categories** per user.
- Track **investments** — reksa dana positions plus gold and silver holdings,
  each with a fixed cost basis and a separately-updatable current unit price.
  Kept out of `source` balances, which is why the clients label those "Liquid".
- Track **debts** and their settlement status.
- Read the **app settings** owned by `login-api` (category IDs used for transfers,
  recounts, and debt bookkeeping — consumed by `post_earning_api_v2` and
  `post_spending_api_v2` to special-case those categories). Fetched over HTTP and
  cached in-process for 60s; a stale cache is preferred over failing a write.
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
  handlers/             # request handlers (sources, earnings, spendings, debts, sync, swagger)
  helper/               # DB connection, JWT, response codes, login-api settings client
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
| `LOGIN_API_BASE` | `http://127.0.0.1:3002` | Base URL of `login-api`, which owns `app_settings`. Read via `GET {LOGIN_API_BASE}/api/settings`. |
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
LOGIN_API_BASE=http://127.0.0.1:3002
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

- `GET/POST/DELETE /api/user/source[-balance|/...]` — sources & balances
- `GET/POST /api/user/earnings`, `GET/POST/DELETE .../earning-categories[/{category}]`
- `GET/POST /api/user/spendings`, `GET/POST/DELETE .../spending-categories[/{category}]`
- `GET /api/user/spending-details[?spending_id=]`, `GET /api/user/spendings/{spending_id}/details`
  — line items ("transaction detail") of a spending. Written by passing an
  optional `details[]` array to `POST /api/user/spendings`; if `total_amount` is
  `0` the total is derived from the items. Used by the Flutter receipt scanner,
  which OCRs a printed price list and posts the rows the user confirmed.
- `GET/POST/DELETE /api/user/investments[/{investment_id}]` — investment
  holdings. `kind` is one of `mutual_fund`, `gold`, `silver`; `units` and the
  unit prices mean units + NAB for a fund and grams + price-per-gram for metal.
  `POST` upserts on the client-generated `investment_id` so offline writes can
  be replayed safely.
- `PUT /api/user/investments/{investment_id}/price` — record a fresh valuation
  only. Deliberately cannot touch `units` or `buy_unit_price`, so a bad price
  refresh can never rewrite what was actually bought.
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
