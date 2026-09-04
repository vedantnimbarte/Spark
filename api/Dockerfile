# Build the control plane.
#
# sqlx checks queries against a live database at compile time, which does not
# exist in a build container, so the build runs against the committed .sqlx
# offline cache. Regenerate it with `cargo sqlx prepare` whenever a query or the
# schema changes, or this build fails.
FROM rust:1-slim-bookworm AS build
WORKDIR /src

RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config \
 && rm -rf /var/lib/apt/lists/*

# Dependencies first, so editing source does not re-download the world.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs \
 && cargo build --release --locked \
 && rm -rf src

COPY .sqlx ./.sqlx
COPY migrations ./migrations
COPY src ./src
ENV SQLX_OFFLINE=true
# Touch so cargo does not reuse the placeholder build artifacts.
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 spark
COPY --from=build /src/target/release/spark-api /usr/local/bin/spark-api

USER 10001
EXPOSE 8080
ENV BIND_ADDR=0.0.0.0:8080
CMD ["spark-api"]
