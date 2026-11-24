FROM rust:1.100.0-alpine3.24 AS build
RUN apk upgrade --no-cache \
    && apk add --no-cache build-base
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY worker ./worker
COPY storage/migrations ./storage/migrations
RUN cargo build --locked --release -p station-api
RUN mkdir -p /runtime/storage/locks /runtime/storage/sqlite /runtime/worker/crawler/temp_assets

FROM cgr.dev/chainguard/static:latest@sha256:54b5f14edad6f2554774d499539e837588a501bd5bf22ac6cd97b1068c35d44d
WORKDIR /app
COPY --from=build --chown=100:101 /runtime/ /app/
COPY --from=build /app/target/release/station-api /usr/local/bin/station-api
USER 100:101
EXPOSE 3212
CMD ["/usr/local/bin/station-api"]
