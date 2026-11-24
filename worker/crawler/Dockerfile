FROM rust:1.100.0-alpine3.24 AS build
RUN apk add --no-cache build-base
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY worker ./worker
COPY storage ./storage
RUN cargo build --locked --release -p station-crawler

FROM alpine:3.24.4
RUN apk upgrade --no-cache \
    && apk add --no-cache ca-certificates
WORKDIR /app
COPY --from=build /app/target/release/station-crawler /usr/local/bin/station-crawler
CMD ["station-crawler", "--once"]
