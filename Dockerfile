FROM --platform=$BUILDPLATFORM rust:alpine AS builder

WORKDIR /app

RUN apk add --no-cache musl-dev

COPY . .

ARG TARGETPLATFORM
ARG TARGETARCH
RUN case "$TARGETARCH" in \
    "amd64") TARGET="x86_64-unknown-linux-musl" ;; \
    "arm64") TARGET="aarch64-unknown-linux-musl" ;; \
    *) echo "Unsupported architecture: $TARGETARCH" && exit 1 ;; \
    esac && \
    rustup target add $TARGET && \
    cargo build --release --target $TARGET

# main image
FROM alpine:latest

ARG BIN_NAME=crabdis
ARG TARGETARCH

COPY --from=builder /app/target/*/release/${BIN_NAME} /usr/local/bin/${BIN_NAME}

EXPOSE 6379

ENTRYPOINT [ "/usr/local/bin/crabdis"]

