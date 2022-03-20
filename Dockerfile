FROM rust:1.59.0 AS builder

# Changes the in container working directory
WORKDIR /app

# Installs all the needed dependencies for our linking setup
RUN apt update && apt install lld clang -y

# Copys the source code in the container
COPY . .

ENV SQLX_OFFLINE true

# Builds our app
RUN cargo build --release

FROM debian:bullseye-slim AS runtime

WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configuration configuration

ENV ZERO2PROD_APP_ENVIRONMENT production
# Starts the app
ENTRYPOINT ["./zero2prod"]