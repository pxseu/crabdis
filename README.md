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

This will start the server on `127.0.0.1:6379`. You can change the address and port with the `--address` and `--port` flags.

# License

This project is licensed under the [MIT License](LICENSE).
