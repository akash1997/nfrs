FROM rust:1.91-slim-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# Install trunk and wasm target
RUN cargo install trunk
RUN rustup target add wasm32-unknown-unknown

COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching layer!
# Only build client dependencies to avoid building server deps (which don't support WASM)
RUN cargo chef cook --release --target wasm32-unknown-unknown --package nfrs_client --recipe-path recipe.json

# Build application
COPY . .

# Build argument for server address
ARG NFRS_SERVER_ADDR
ENV NFRS_SERVER_ADDR=${NFRS_SERVER_ADDR}

WORKDIR /app/nfrs_client
RUN trunk build --release

# Runtime stage
FROM nginx:alpine
COPY --from=builder /app/nfrs_client/dist /usr/share/nginx/html
COPY cert.pem /etc/nginx/ssl/cert.pem
COPY key.pem /etc/nginx/ssl/key.pem
COPY nfrs_client/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80 443
CMD ["nginx", "-g", "daemon off;"]
