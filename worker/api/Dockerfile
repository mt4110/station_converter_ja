FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml ./
COPY worker ./worker
COPY storage ./storage
RUN cargo build --release -p station-api

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /app/target/release/station-api /usr/local/bin/station-api
EXPOSE 3212
CMD ["station-api"]
