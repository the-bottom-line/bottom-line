# Stage 1: Prepare rust environment for plan and build stage
FROM rust:latest AS chef
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install cargo-chef
WORKDIR /app

# Stage 2: Planning, let chef plan dependencies
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Build
FROM chef AS builder

# Build dependencies
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --target x86_64-unknown-linux-musl

# Rest of the owl
COPY . .
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl --bin server

# Stage 4: Runtime
FROM alpine:3.20 AS runtime
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/server .
EXPOSE 3000
RUN chmod +x server
CMD ["./server"]
