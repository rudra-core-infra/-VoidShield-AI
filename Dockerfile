FROM rust:1.76-alpine as builder
WORKDIR /usr/src/tier1-dropin-shield
RUN apk add --no-cache musl-dev

# Cache dependencies
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf target/release/deps/tier1_dropin_shield* target/release/tier1-dropin-shield*

# Build actual source (touch lagana zaroori hai bug rokne ke liye)
COPY src ./src
RUN touch src/main.rs
RUN cargo build --release

FROM alpine:3.19
RUN apk add --no-cache ca-certificates tzdata
WORKDIR /app
COPY --from=builder /usr/src/tier1-dropin-shield/target/release/tier1-dropin-shield /usr/local/bin/tier1-dropin-shield

ENV UPSTREAM_URL="https://api.openai.com"
ENV PORT="8080"
EXPOSE 8080
ENTRYPOINT ["tier1-dropin-shield"]

