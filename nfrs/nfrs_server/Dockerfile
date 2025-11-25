FROM rust:1.91-slim-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching layer!
RUN cargo chef cook --release --package nfrs_server --recipe-path recipe.json

# Build application
COPY . .
RUN cargo build --release -p nfrs_server

# Runtime stage
FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/nfrs_server /app/nfrs_server
COPY cert.pem /app/cert.pem
COPY key.pem /app/key.pem

EXPOSE 5000/udp
EXPOSE 5001/udp

CMD ["./nfrs_server"]
