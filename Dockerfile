FROM rust:1.76-alpine as builder
WORKDIR /usr/src/tier1-dropin-shield
RUN apk add --no-cache musl-dev

# Cache dependencies
COPY Cargo.toml .
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/tier1_dropin_shield*

# Build actual source
COPY src ./src
RUN cargo build --release

FROM alpine:3.19
RUN apk add --no-cache ca-certificates
WORKDIR /app
COPY --from=builder /usr/src/tier1-dropin-shield/target/release/tier1-dropin-shield /usr/local/bin/
EXPOSE 8080
CMD ["tier1-dropin-shield"]
