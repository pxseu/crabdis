# crabdis

> It's like Redis but a bit rusty...

# What?

This is a simple in-memory key-value store written in Rust. It's somewhat compatible with Redis via the [RESP](https://redis.io/docs/reference/protocol-spec/) protocol, but it's not a drop-in replacement. A lot of commands are missing and stuff might not work as expected.

> [!NOTE]
> While it's technically possible to run `crabdis` in production, it's not recommended, use at your own risk!

# Why?

I like tinkering with stuff I use and Redis is a great tool. It was started when the License fiasco happened and I wanted to write my own Redis-compatible server in Rust. This project works with notable clients like [ioredis](https://github.com/luin/ioredis) and [Bun](https://bun.com/docs/runtime/redis).

> [!IMPORTANT]
> Bun support is only available since version `0.1.25` of `crabdis` due to incorrect RESP3 support in earlier versions. As always, use the latest version of `crabdis` and `bun` to get the best experience.

# Installation

You can find binaries on the [releases page](https://github.com/pxseu/crabdis/releases). Or you can build it yourself with `cargo build --release`.

If you want to install it with cargo, you can do so with `cargo install crabdis`.

There is also a Docker image available on [Docker Hub](https://hub.docker.com/r/pxseu/crabdis).

# Usage

By default, `crabdis` will listen on all addresses on port 6379. This is the same as running `crabdis --address :: --port 6379`. This has been chosen because [Railway](https://docs.railway.com/guides/private-networking#listen-on-ipv6)'s internal networking used to be IPv6 only.

```sh
crabdis
```

# TODO / Missing Features

- [x] Basic RESP protocol implementation
- [x] GET, SET, DEL, EXISTS, KEYS, FLUSHDB
- [x] COMMAND / COMMAND DOCS (so ioredis works)
- [x] SET arguments (EX, PX, NX, XX) + SETEX, PSETEX
- [x] Hash Command family (HGETALL, HSET)
- [x] Pub/Sub support (PUBLISH, SUBSCRIBE, UNSUBSCRIBE)
- [x] Additional commands (INCR, MGET, MSET, TYPE, SCAN, SELECT, RENAMENX, INFO, HELLO)
- [x] Persistence
- [ ] List commands
- [ ] Set commands
- [ ] Sorted Set commands

# Benchmarks

Below are micro-benchmarks for core Value operations and RESP serialization/deserialization (run with `cargo bench`).

It's pretty fast actually.

# License

This project is licensed under the [MIT License](LICENSE).
