FROM rust:latest AS builder

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
RUN cargo build --release --no-default-features

# --- Runtime stage ---
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r cookcli && useradd -r -g cookcli -d /home/cookcli -s /sbin/nologin cookcli

# Copy binary
COPY --from=builder /usr/src/cookcli/target/release/cook /usr/local/bin/cook

# Copy seed recipes
COPY seed/ /recipes/

RUN chown -R cookcli:cookcli /recipes

USER cookcli

EXPOSE 9080

ENTRYPOINT ["cook", "server", "--host", "/recipes"]
