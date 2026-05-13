FROM rust:1-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y build-essential pkg-config libssl-dev

# Install Dioxus CLI for building frontend assets
RUN cargo install dioxus-cli --locked

# Copy workspace sources
COPY . .

# Build frontend assets (WASM + JS bundles)
# This produces the public/ directory with Dioxus client-side bundle
RUN cd crates/yoda-web && \
    dx build --release

# Build the server binary with frontend assets available
# The server expects public/ to be adjacent to the binary at runtime
RUN cargo build --release -p yoda-web --features server

# Copy the built public assets to be adjacent to the binary in the runtime image
RUN mkdir -p /app/yoda-web-release && \
    cp /app/target/release/yoda-web /app/yoda-web-release/ && \
    if [ -d /app/target/dx/yoda-web/release/web/public ]; then \
    cp -r /app/target/dx/yoda-web/release/web/public /app/yoda-web-release/public; \
    elif [ -d /app/dist/public ]; then \
    cp -r /app/dist/public /app/yoda-web-release/public; \
    else \
    echo "ERROR: could not find built Dioxus public assets" && \
    exit 1; \
    fi

FROM debian:trixie-slim AS runtime

WORKDIR /app

# Provide sensible defaults that can be overridden at `docker run` time.
ENV YODA_IMAGE_BASE_PATH=/data/images
ENV YODA_LABEL_BASE_PATH=/data/labels
ENV YODA_HOST=0.0.0.0
ENV YODA_PORT=8080

# Copy the compiled web server and public assets from the build stage
COPY --from=builder /app/yoda-web-release /usr/local/yoda-web

EXPOSE 8080

CMD ["/usr/local/yoda-web/yoda-web"]
