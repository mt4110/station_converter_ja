FROM rust:1.100.0-alpine3.24 AS build
RUN apk upgrade --no-cache \
    && apk add --no-cache build-base
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY worker ./worker
COPY storage/migrations ./storage/migrations
RUN cargo build --locked --release -p station-api

FROM alpine:3.24.4
RUN apk upgrade --no-cache \
    && apk add --no-cache ca-certificates \
    && addgroup -S station \
    && adduser -S -D -H -h /app -G station station
WORKDIR /app
RUN mkdir -p storage/locks storage/sqlite worker/crawler/temp_assets \
    && chown -R station:station /app
COPY --from=build /app/target/release/station-api /usr/local/bin/station-api
USER station
EXPOSE 3212
CMD ["station-api"]
