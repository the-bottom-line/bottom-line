# Setup environment
FROM rust AS builder
WORKDIR /app

RUN rustup target add x86_64-unknown-linux-musl

# Build dependencies first to cache in the layer
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN rm -rf src

# Rest of the owl
COPY ./src ./src
RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime
FROM alpine:3.20
WORKDIR /app

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/bottom-line .

RUN chmod +x bottom-line

CMD ["./bottom-line"]
