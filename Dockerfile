# Cargo profile for the binary. `release` (fat LTO) is what gets published;
# `docker-dev` (thin LTO, parallel codegen) rebuilds several times faster and is
# meant for images built to try a change locally: `make docker-build-dev`.
ARG CARGO_PROFILE=release

# --- Front-end assets ---
# A stage of its own so the Rust builder needs no Node.js, and so Rust-only
# edits don't rebuild the CSS and JS.
FROM node:lts-bookworm-slim AS assets

WORKDIR /usr/src/cookcli

COPY package.json package-lock.json ./
RUN --mount=type=cache,target=/root/.npm npm ci --no-audit --no-fund

# Only what Tailwind scans (the @source lines in static/css/input.css) and
# esbuild bundles.
COPY static/ static/
COPY templates/ templates/
COPY src/web/templates.rs src/web/templates.rs
RUN npm run build-css && npm run build-js

# --- Rust dependencies ---
# cargo-chef compiles the dependency graph from a recipe that only changes with
# Cargo.toml/Cargo.lock, so the result is an ordinary image layer: it survives
# in the registry/GHA layer cache, unlike a `--mount=type=cache` target dir.
FROM rust:bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /usr/src/cookcli

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG CARGO_PROFILE

# Server + lsp (the editor's /ws/lsp bridge spawns `cook lsp`), but without
# self-update (useless in a container) or import (would pull in reqwest).
COPY --from=planner /usr/src/cookcli/recipe.json recipe.json
RUN cargo chef cook --profile "$CARGO_PROFILE" --recipe-path recipe.json \
    --no-default-features --features server,lsp

COPY . .
COPY --from=assets /usr/src/cookcli/static/css/output.css static/css/output.css
COPY --from=assets /usr/src/cookcli/static/js/editor.bundle.js static/js/editor.bundle.js
RUN cargo build --profile "$CARGO_PROFILE" --no-default-features --features server,lsp \
    && cp "target/$CARGO_PROFILE/cook" /usr/local/bin/cook

# --- Runtime stage ---
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user with well-known UID/GID (1000:1000)
# This matches the default first user on most Linux systems,
# reducing permission issues with mounted volumes.
# Override in docker-compose.yml with `user: "YOUR_UID:YOUR_GID"` if needed.
RUN groupadd -g 1000 cookcli && useradd -u 1000 -g cookcli -d /home/cookcli -s /sbin/nologin cookcli

# Copy binary
COPY --from=builder /usr/local/bin/cook /usr/local/bin/cook

# Copy seed recipes as defaults (override by mounting your own recipes at /recipes)
COPY seed/ /recipes/

# Copy entrypoint script
COPY docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

RUN chown -R cookcli:cookcli /recipes

USER cookcli

VOLUME /recipes
EXPOSE 9080

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
CMD ["cook", "server", "/recipes", "--host"]
