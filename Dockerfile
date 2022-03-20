FROM ekidd/rust-musl-builder:latest AS builder


# Copys the source code in the container
ADD . /home/rust/src
RUN sudo chown rust: /home/rust -R

ENV SQLX_OFFLINE true

# Builds our app
RUN cargo build --release

FROM alpine:latest AS runtime

WORKDIR /app
RUN apk --no-cache add ca-certificates

COPY --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/zero2prod zero2prod
COPY configuration configuration

ENV ZERO2PROD_APP_ENVIRONMENT production
# Starts the app
ENTRYPOINT ["./zero2prod"]