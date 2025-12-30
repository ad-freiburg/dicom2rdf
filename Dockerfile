FROM rust:1.91.0-alpine AS builder
RUN apk add --no-cache musl-dev make
WORKDIR /build
COPY Cargo.toml Cargo.lock .
COPY crates crates
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-target,target=/build/target \
    cargo build --release -p convert && \
    cargo build --release -p construct && \
    cargo build --release -p populate-prefixes && \
    cp target/release/convert target/release/construct target/release/populate-prefixes /tmp

FROM alpine:3 AS convert
ENV RUST_LOG=info
COPY --from=builder /tmp/convert /app/
COPY config.toml /app/
CMD ["/app/convert", "--config", "/app/config.toml", "--input-dir", "/input", "--output-dir", "/ttl"]

FROM docker.io/adfreiburg/qlever@sha256:04903551c4c8d27f8ba13e6e67906d30116e5b1ebf83f7716babbad61751b1b6 AS construct
USER root
RUN apt-get update && apt-get install -y jq && rm -rf /var/lib/apt/lists/*
USER qlever
COPY --from=builder /tmp/construct /app/
COPY config.toml /app/
COPY ttl-static/ /ttl-static
COPY scripts/construct.sh /scripts/
CMD ["-c", "RUST_LOG=info bash /scripts/construct.sh"]

FROM docker.io/adfreiburg/qlever-ui@sha256:a28aabe224d391f293a7e95f79bf5ae55ff1b87c51bf1e2c10900563ac7990a3 AS qlever-ui
RUN apk add --no-cache gettext
ENV RUST_LOG=info
COPY --from=builder /tmp/populate-prefixes .
COPY config.toml /app/
COPY Qleverfile-ui.template.yml /app/
RUN ./populate-prefixes \
    --config config.toml \
    --qleverfile-ui /app/Qleverfile-ui.template.yml
COPY scripts/entrypoint-ui.sh /scripts/entrypoint.sh
ENTRYPOINT ["bash", "/scripts/entrypoint.sh"]

FROM alpine:3 AS update-ttl-static
RUN apk add --no-cache bash curl jq libxml2-utils py3-pip \
    && pip install --break-system-packages yq
COPY scripts/update_*.sh /scripts/
CMD ["bash", "-c", "/scripts/update_dcm_tag_labels.sh && /scripts/update_dcm_concept_code_labels.sh && /scripts/update_sop_class_uid_labels.sh"]
