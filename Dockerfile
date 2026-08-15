# syntax=docker/dockerfile:1.7

FROM rust:1.97.1-bookworm@sha256:0e2bcaef56d041a486784e54104a81aebe0da44bd03019bd70bc0401e42e4a97 AS builder

WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
COPY apps/api/Cargo.toml apps/api/Cargo.toml

RUN cargo fetch --locked

COPY apps/api/src apps/api/src
COPY apps/api/migrations apps/api/migrations

RUN cargo build --release --locked --offline --bin bycard-api --bin container-healthcheck --bin database-operations && \
    install -D -m 0755 target/release/bycard-api /artifacts/bycard-api && \
    install -D -m 0755 target/release/container-healthcheck /artifacts/container-healthcheck && \
    install -D -m 0755 target/release/database-operations /artifacts/database-operations && \
    strip /artifacts/bycard-api /artifacts/container-healthcheck /artifacts/database-operations

FROM debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241 AS operations

LABEL org.opencontainers.image.source="https://github.com/underthesamename/Bycard" \
      org.opencontainers.image.description="Bycard database operations"

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /artifacts/database-operations /usr/local/bin/database-operations

ENV APP_ENV=production

USER 65532:65532
ENTRYPOINT ["/usr/local/bin/database-operations"]
CMD ["migrate"]

FROM debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241 AS runtime

LABEL org.opencontainers.image.source="https://github.com/underthesamename/Bycard" \
      org.opencontainers.image.description="Bycard Rust API"

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /artifacts/bycard-api /usr/local/bin/bycard-api
COPY --from=builder /artifacts/container-healthcheck /usr/local/bin/container-healthcheck

ENV APP_ENV=production \
    API_HOST=0.0.0.0 \
    API_PORT=8080 \
    RUST_LOG=bycard_api=info,tower_http=info

EXPOSE 8080
USER 65532:65532
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/container-healthcheck"]

ENTRYPOINT ["/usr/local/bin/bycard-api"]
