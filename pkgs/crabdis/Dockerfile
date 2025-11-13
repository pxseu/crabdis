FROM --platform=$BUILDPLATFORM rust:alpine AS builder

WORKDIR /app

RUN apk add --no-cache musl-dev

COPY . .

RUN cargo build --release

# main image
FROM alpine

ARG BIN_NAME=crabdis

COPY --from=builder /app/target/release/${BIN_NAME} /usr/local/bin/${BIN_NAME}

EXPOSE 6379

LABEL org.opencontainers.image.source="https://github.com/pxseu/crabdis"

ENTRYPOINT [ "/usr/local/bin/crabdis" ]

