# Docker

Compose runs **MariaDB 11** and the **tfs-rust** game server. Schema is SQLx migrations on first boot (`TFS_MIGRATIONS_DIR`), not a dumped `schema.sql`.

Stop a host `./scripts/run_server.sh` first — it binds the same login/game ports.

## Quick start (prebuilt image)

After `main` is pushed, GitHub Actions publishes `ghcr.io/tezoze/tfs-rust:latest` (`.github/workflows/docker.yml`). Clone for Compose wiring, then pull:

```bash
git clone https://github.com/Tezoze/TFS-RUST.git
cd TFS-RUST
cp .env.example .env   # optional
docker compose pull
docker compose up
```

Detached: `docker compose up -d`.

**Package visibility (repo owner, once):** GitHub → **Packages** → `tfs-rust` → **Package settings** → **Change visibility** → **Public**. Until then, others need `docker login ghcr.io` with a PAT that has `read:packages`.

**Forks:** set `TFS_IMAGE=ghcr.io/<your-lowercase-user>/tfs-rust:latest` in `.env` after that fork’s `main` workflow has published.

## Compile in Docker (no GHCR)

```bash
cp .env.example .env   # optional; defaults are local-dev passwords
docker compose up --build
```

Rebuild after Rust or datapack changes:

```bash
docker compose up --build
```

## Stop / logs

```bash
docker compose down          # stop containers; MariaDB volume is kept
docker compose logs -f tfs   # server log
docker compose logs -f db
```

`down -v` deletes the `tfs-mariadb` volume (wipes the database).

## Ports and env

| Item | Role |
|------|------|
| `7171` / `7172` | Login / game (published on the host) |
| `DATABASE_URL` | Compose service `db` (overrides `config.lua` `mysql*`) |
| `TFS_PUBLIC_IP` | Address **clients** use after login (default `127.0.0.1`) |
| `TFS_IMAGE` | Image to run (`ghcr.io/tezoze/tfs-rust:latest` unless you `--build`) |
| `MYSQL_PASSWORD` / `MYSQL_ROOT_PASSWORD` | MariaDB (defaults match Compose) |
| `RUST_LOG` | Tracing filter |

MariaDB is **not** published on host `3306` (avoids clashing with a local daemon).

Copy `.env.example` → `.env` next to `docker-compose.yml` to override those values.

### Clients on another machine

Set `TFS_PUBLIC_IP` in `.env` to the **host’s** LAN (or public) IP — not the container IP. The server already binds `0.0.0.0` when `bindOnlyGlobalAddress` is false (`config.lua.dist`).

## Dev account

Compose creates the MariaDB user only. A one-shot `seed` service waits for the `accounts` table (migrations), then inserts:

- Account **`1`** / password **`1`**
- Characters: **God** (group 6), **Master Sorcerer**, **Elder Druid**, **Royal Paladin**, **Elite Knight** (level 100)

SQL: `docker/seed_dev_account.sql`. Password is SHA1 in the dump, bcrypt on first login. Idempotent — existing rows are left alone.

Client: 7.72-compatible, login port **7171**.

## Image layout

Multi-stage `Dockerfile`: `rust:1.96-bookworm` builder (`SQLX_OFFLINE=true`, committed `.sqlx/`, vendored LuaJIT via `mlua`) → `debian:bookworm-slim` runtime as user `tfs`. Copies `config.lua.dist` → `/srv/config.lua`, `key.pem`, `data/`, and `crates/tfs-rust-db/migrations`.

`docker/entrypoint.sh` waits for `TFS_DB_WAIT_HOST` (Compose: `db`) then execs `tfs-rust`.

## Live config / datapack

Optional bind-mounts are commented in `docker-compose.yml`:

```yaml
# volumes:
#   - ./config.lua:/srv/config.lua:ro
#   - ./data:/srv/data:ro
```

Uncomment after `cp config.lua.dist config.lua`. Image rebuild is still required for Rust binary changes.

## Not production

Default DB passwords, `key.pem` in the image, and account `1`/`1` are for local/dev. Do not expose that stack on the public internet as-is.

Native (non-Docker) setup: [COMPILING.md](COMPILING.md).
