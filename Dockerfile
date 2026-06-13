FROM rust:1 AS builder
WORKDIR /app
ARG APP_VERSION=dev
ENV APP_VERSION=$APP_VERSION
COPY Cargo.toml build.rs ./
COPY src ./src
COPY static ./static
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/alertview /usr/local/bin/alertview
EXPOSE 8080
CMD ["alertview", "/config/config.yaml"]
