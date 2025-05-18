# crabdis

> It's like Redis but a bit rusty...

# What?

This is a simple in-memory key-value store written in Rust. It's somewhat compatible with Redis via the [RESP](https://redis.io/docs/reference/protocol-spec/) protocol, but it's not a drop-in replacement. A lot of commands are missing and stuff might not work as expected.

Please don't use this in production. Or do, I'm not your mom. But don't blame me if it eats your data.

# Why?

I wanted to write Redis but multi-threaded and in Rust. This is the result.
Works? Kinda. Is it good? Maybe. Is it fast? Yes.

# Installation

You can find binaries on the [releases page](https://github.com/pxseu/crabdis/releases). Or you can build it yourself with `cargo build --release`.

If you want to install it with cargo, you can do so with `cargo install crabdis`.

There is also a Docker image available on [Docker Hub](https://hub.docker.com/r/pxseu/crabdis).

# Usage

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
- [ ] Persistence
- [ ] More Hash commands (HGET, HDEL, etc.)
- [ ] List commands
- [ ] Set commands
- [ ] Sorted Set commands

This will start the server on `127.0.0.1:6379`. You can change the address and port with the `--address` and `--port` flags.

# Benchmarks

Below are micro-benchmarks for core Value operations and RESP serialization/deserialization (run with `cargo bench`).

| File                      | Operation                 | Time (ns) |
| ------------------------- | ------------------------- | --------- |
| value_benchmarks.rs       | create_string             | 16.15     |
|                           | create_integer            | 2.14      |
|                           | create_multi              | 54.0      |
|                           | create_map                | 105.3     |
|                           | is_some                   | 0.581     |
|                           | is_none                   | 0.443     |
|                           | inner                     | 20.0      |
| value_resp2_benchmarks.rs | resp2_serialize_string    | 161       |
|                           | resp2_serialize_integer   | 102       |
|                           | resp2_serialize_nil       | 54        |
|                           | resp2_serialize_multi     | 511       |
|                           | resp2_serialize_map       | 844       |
|                           | resp2_serialize_expire    | 97        |
|                           | resp2_deserialize_string  | 294       |
|                           | resp2_deserialize_integer | 228       |
|                           | resp2_deserialize_nil     | 232       |
|                           | resp2_deserialize_multi   | 598       |
| value_resp3_benchmarks.rs | resp3_serialize_map       | 805       |
|                           | resp3_serialize_set       | 503       |
|                           | resp3_serialize_push      | 591       |
|                           | resp3_serialize_error     | 188       |
|                           | resp3_deserialize_map     | 826       |
|                           | resp3_deserialize_set     | 532       |
|                           | resp3_deserialize_push    | 568       |
|                           | resp3_deserialize_error   | 269       |

**Total (sum of all measured times): ≈ 7602 ns**

_(Lower is better. All times are approximate and measured on a Mac, see benches/ for details.)_

# License

This project is licensed under the [MIT License](LICENSE).
