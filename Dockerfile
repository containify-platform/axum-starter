FROM rust:1-slim-trixie AS build
WORKDIR /src
# Build the dependencies on their own first, so they stay cached until
# Cargo.toml or Cargo.lock changes.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && \
    cargo build --release --locked && \
    rm -rf src
COPY . .
# COPY keeps the files' times, which can be older than the placeholder build:
# touch main.rs so Cargo rebuilds the crate itself.
RUN touch src/main.rs && cargo build --release --locked

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    useradd --system --no-create-home app
COPY --from=build /src/target/release/axum-starter /usr/local/bin/axum-starter
USER app
EXPOSE 8080
CMD ["axum-starter"]
