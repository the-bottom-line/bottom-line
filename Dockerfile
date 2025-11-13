FROM rust:latest AS chef
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer
RUN cargo chef cook --release --recipe-path recipe.json --target x86_64-unknown-linux-musl

# Build application
COPY . .
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl --bin server

# Runtime
FROM alpine:3.20
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/server .
EXPOSE 3000
RUN chmod +x server
CMD ["./server"]
