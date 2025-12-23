FROM rust:1.90-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
  pkg-config \
  libssl-dev

COPY . .

RUN cargo build --release --target x86_64-unknown-linux-gnu --manifest-path "$PWD/report_generator/Cargo.toml"

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
  libssl-dev \
  pkg-config \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/report_generator /app/
COPY --from=builder /app/config_files/deploy_config.yml /app/config_files/deploy_config.yml
COPY --from=builder /app/report_generator/template_files /app/template_files

WORKDIR /app

ENTRYPOINT ["./report_generator"]
