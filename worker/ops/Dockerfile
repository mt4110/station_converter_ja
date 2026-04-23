FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY worker ./worker
COPY storage/migrations ./storage/migrations
RUN cargo build --locked --release -p station-ops

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system station \
    && useradd --system --gid station --home-dir /app --no-create-home station
WORKDIR /app
RUN mkdir -p storage/locks storage/sqlite worker/crawler/temp_assets \
    && chown -R station:station /app
COPY --from=build /app/target/release/station-ops /usr/local/bin/station-ops
USER station
ENTRYPOINT ["station-ops"]
CMD ["--help"]
