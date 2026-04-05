FROM rust:1.80-slim AS builder

WORKDIR /app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates sqlite3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/mermaduckle-server /app/mermaduckle-server
COPY --from=builder /app/crates/server/static /app/static

EXPOSE 3000

CMD ["./mermaduckle-server"]