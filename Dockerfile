FROM rust:bookworm AS builder

# Install Node.js for Tailwind CSS and esbuild
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get update \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/cookcli

# Install npm dependencies first (cache layer)
COPY package.json package-lock.json* ./
RUN npm install

# Copy source
COPY . .

# Build CSS and JS assets
RUN npm run build-css && npm run build-js

# Build Rust binary without self-update feature
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/cookcli/target \
    cargo build --release --no-default-features \
    && cp target/release/cook /usr/local/bin/cook

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
