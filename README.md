# crabdis

> It’s like Redis but a bit rusty.

`crabdis` is a small, in-memory key-value store written in Rust that speaks the Redis protocol (RESP).

> [!NOTE]
> This project is experimental. Use in production only if you’re comfortable with sharp edges.

## What’s implemented

- RESP2 by default, with optional RESP3 via `HELLO 3`
- Core key/value operations (strings + integers)
- Hashes
- Key expiry (EXPIRE/TTL/etc.)
- Pub/Sub (RESP2 arrays / RESP3 push)
- RDB persistence (load on boot + `SAVE`/`BGSAVE` + periodic auto-save)

Known to work with clients like `redis-cli`, `ioredis`, and Bun’s Redis client, but compatibility is not guaranteed.

## Install

- **Releases**: download prebuilt binaries from `https://github.com/pxseu/crabdis/releases`
- **Cargo**: `cargo install crabdis`
- **Docker**: `pxseu/crabdis` on Docker Hub

## Quickstart

Run locally:

```sh
crabdis
```

Connect with `redis-cli`:

```sh
redis-cli -p 6379 ping
```

### Docker

The Docker image disables persistence by default (it runs with `--save ""`).

```sh
docker run --rm -p 6379:6379 pxseu/crabdis:latest
```

To enable persistence, mount a volume and pass `--dir` and one or more `--save` points:

```sh
docker run --rm -p 6379:6379 \
  -v crabdis_data:/data \
  pxseu/crabdis:latest \
  --dir /data --save "60 1"
```

(See `docker-compose.yml` for an example setup.)

## Configuration

Run `crabdis --help` for the full list. Common flags:

- `--address` / `--port`: bind address and port (Unix defaults to `::`, Windows defaults to `127.0.0.1`)
- `--threads`: Tokio worker threads
- `--verbose`: enable verbose logging
- `--dir` / `--dbfilename`: RDB location (defaults to `./dump.rdb`)
- `--save "SECONDS CHANGES"`: auto-save points; pass `--save ""` to disable RDB persistence
  - If you don’t specify any `--save` points, Crabdis uses defaults: `3600 1`, `300 100`, `60 10000`

## Supported commands

Crabdis supports a (growing) subset of Redis commands. Highlights:

- **Core**: `GET`, `SET` (incl. `EX`/`PX`/`NX`/`XX`), `DEL`, `EXISTS`, `KEYS`, `SCAN`, `TYPE`, `DBSIZE`, `FLUSHDB`, `SELECT`, `RENAMENX`
- **Multi-key**: `MGET`, `MSET`
- **Counters**: `INCR`, `DECR`
- **Expiry**: `EXPIRE`, `TTL`, `PTTL`, `PERSIST`, `SETEX`, `PSETEX`
- **Hashes**: `HSET`, `HGET`, `HGETALL`, `HDEL`, `HEXISTS`, `HLEN`
- **Pub/Sub**: `PUBLISH`, `SUBSCRIBE`, `UNSUBSCRIBE`
- **Server/Protocol**: `PING`, `QUIT`, `INFO`, `HELLO`, `COMMAND` / `COMMAND DOCS`, `CLIENT`
- **Persistence**: `SAVE`, `BGSAVE`, `LASTSAVE`

For client compatibility, prefer discovering capabilities via `COMMAND`/`COMMAND DOCS` rather than assuming Redis parity.

## Not implemented (yet)

- Lists, Sets, Sorted Sets
- Replication, clustering, ACLs
- Lua scripting, transactions, modules

## Development

- Build: `cargo build --release`
- Run: `cargo run -p crabdis`
- Test: `cargo test`
- Bench (core values + RESP): `cargo bench -p crabdis-core`

## License

MIT License, see [`LICENSE`](./LICENSE).
