FROM --platform=$BUILDPLATFORM rust:alpine AS builder

WORKDIR /app

RUN apk add --no-cache musl-dev

COPY . .

RUN cargo build --release --locked

# main image
FROM alpine

ARG BIN_NAME=crabdis

COPY --from=builder /app/target/release/${BIN_NAME} /usr/local/bin/${BIN_NAME}

EXPOSE 6379

LABEL org.opencontainers.image.title=${BIN_NAME}
LABEL org.opencontainers.image.description="It's like Redis but a bit rusty..."
LABEL org.opencontainers.image.source="https://github.com/pxseu/crabdis"
LABEL org.opencontainers.image.url="docker.io/pxseu/crabdis"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.vendor="pxseu"

ENTRYPOINT [ "/usr/local/bin/crabdis" ]
CMD [ "--save", ""]
