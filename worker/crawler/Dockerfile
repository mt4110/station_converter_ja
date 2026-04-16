FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml ./
COPY worker ./worker
COPY storage ./storage
RUN cargo build --release -p station-crawler

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /app/target/release/station-crawler /usr/local/bin/station-crawler
CMD ["station-crawler", "--once"]
