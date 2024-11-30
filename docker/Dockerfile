# Use the official Rust image as the base
FROM rust:latest

# Install Node.js (latest version)
RUN curl -fsSL https://deb.nodesource.com/setup_current.x | bash - \
    && apt-get update \
    && apt-get install -y nodejs

# Set the working directory in the container
WORKDIR /usr/src/cookcli

# Clone the cookcli repository
RUN git clone https://github.com/cooklang/cookcli.git .

# Install any JavaScript dependencies (if applicable)
WORKDIR /usr/src/cookcli/ui
RUN npm install && npm run build

# Build the project (both Rust and Node dependencies)
WORKDIR /usr/src/cookcli/src
RUN cargo build --release

# Add the cook binary to the PATH
ENV PATH="/usr/src/cookcli/target/release:${PATH}"

# Expose port 9080 for the web server
EXPOSE 9080

# Switch to the /recipes directory
WORKDIR /recipes

# Run the cook server with --host
CMD ["cook", "server", "--host"]

