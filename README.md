# Axum starter

A minimal Rust web server, ready to deploy on [Containify](https://containify.cloud):

- [Axum](https://github.com/tokio-rs/axum) 0.8 on Tokio
- Postgres through [sqlx](https://github.com/launchbadge/sqlx) with runtime queries, so building needs no database
- Migrations in `migrations/`, compiled into the binary and applied at startup
- `GET /` answers JSON and counts visits in Postgres; `GET /health` checks the database
- Graceful shutdown on `SIGTERM`, so deploys don't cut requests off
- A multi-stage Dockerfile with cached dependencies, shipping the binary on `debian:trixie-slim` as a non-root user

## Run it locally

You need Rust 1.94 or newer and a Postgres database:

```bash
docker run -d --name starter-db -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres:17
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres
cargo run
curl localhost:8080
```

Or with Docker only:

```bash
docker build -t axum-starter .
docker run --rm -p 8080:8080 -e DATABASE_URL=postgres://postgres:postgres@host.docker.internal:5432/postgres axum-starter
```

## Configuration

| Variable | Default | |
| --- | --- | --- |
| `DATABASE_URL` | (required) | `postgres://user:password@host:5432/dbname` |
| `PORT` | `8080` | The port to listen on |
| `HOST` | `::` | The address to listen on; `::` is every interface, IPv6 and IPv4 |
| `MIGRATE_ON_START` | `true` | `false` skips migrations at startup |
| `RUST_LOG` | `info,sqlx=warn` | Log filter, e.g. `debug` |

## Migrations

Add one with `sqlx migrate add <name>` ([sqlx-cli](https://crates.io/crates/sqlx-cli)), or as a
timestamped `.sql` file in `migrations/`. The server applies the new ones when it starts, under
an advisory lock, so replicas starting together don't race. To run them once per deploy instead,
set `MIGRATE_ON_START=false` and use `axum-starter migrate` as the app's pre-deploy command.

To use the compile-time checked `query!` macros, run `cargo sqlx prepare`, commit the `.sqlx`
folder and set `SQLX_OFFLINE=true` in the Dockerfile's build stage.

## Deploy on Containify

Deploy the [Axum starter template](https://containify.cloud/templates/axum): it builds this
repository with its Dockerfile and adds a Postgres database with `DATABASE_URL` already set. To
make it your own, fork this repository and point the app's source at your fork.

## License

MIT
