FROM rust:alpine as builder

WORKDIR /app

RUN apk add --no-cache musl-dev

COPY . .

RUN cargo build --release --target x86_64-unknown-linux-musl

# main image

FROM alpine:latest

ARG BIN_NAME=crabdis

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/${BIN_NAME} /usr/local/bin/${BIN_NAME}

EXPOSE 6379

ENTRYPOINT [ "/usr/local/bin/crabdis"]

