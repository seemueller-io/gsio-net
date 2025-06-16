################################################################
#  Stage 1 – build rustls-cert-gen and generate the certs
################################################################
FROM rust:bookworm AS ssl-step

# ↓ Allow override of SAN / output directory at build time
ARG CERT_DOMAIN=relay.local
ARG OUT_DIR=/app/ssl

# ── deps we need only for the build ───────────────────────────
RUN apt-get update -qq && \
    apt-get install -y --no-install-recommends git ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# ── fetch the rcgen repo (contains the CLI) and build once ────
WORKDIR /src
RUN git clone --depth 1 https://github.com/rustls/rcgen.git
WORKDIR /src/rcgen
RUN cargo run -- -o /app/ssl

################################################################
#  Stage 2 – minimal runtime with the iroh relay
################################################################
FROM n0computer/iroh-relay:v0.28.2

# copy the certs produced in stage 1
COPY --from=ssl-step /app/ssl /app/ssl

# your relay configuration
COPY ./relay-config.toml /app/

# hand off control to the relay
CMD ["--dev"]