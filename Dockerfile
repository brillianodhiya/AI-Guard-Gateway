# Stage 1: Build binary using official Rust image
FROM rust:1.75-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

# Stage 2: Ultra-small minimal runtime container (~15MB)
FROM alpine:latest

RUN apk add --no-cache ca-certificates tzdata

WORKDIR /app
COPY --from=builder /app/target/release/ai-guard-gateway /app/ai-guard-gateway

EXPOSE 8080

CMD ["/app/ai-guard-gateway"]
