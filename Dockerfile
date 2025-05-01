FROM --platform=$BUILDPLATFORM rust:alpine AS builder

WORKDIR /app

RUN apk add --no-cache musl-dev

COPY . .

ARG TARGETPLATFORM
ARG TARGETARCH
RUN TARGET=$([ "$TARGETARCH" = "amd64" ] && echo "x86_64-unknown-linux-musl" || echo "aarch64-unknown-linux-musl") && \
    cargo build --release --target $TARGET

# main image
FROM alpine

ARG BIN_NAME=crabdis
ARG TARGETARCH

COPY --from=builder /app/target/*/release/${BIN_NAME} /usr/local/bin/${BIN_NAME}

EXPOSE 6379

ENTRYPOINT [ "/usr/local/bin/crabdis" ]

