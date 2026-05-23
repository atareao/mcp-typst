# ── Etapa 1: Compilación ──────────────────────────────────────────────────────
FROM docker.io/library/rust:alpine3.23 AS builder

RUN apk add --no-cache --update \
            build-base \
            autoconf \
            gdb \
            musl-dev \
            pkgconfig \
            strace \
            openssl \
            openssl-dev \
            openssl-libs-static

WORKDIR /app

# Cache de dependencias (evita recompilar crates cada vez)
COPY Cargo.toml .
# RUN mkdir src && echo "fn main() {}" > src/main.rs && \
#     cargo build --release && \
#     rm -rf src

# Compilación real
COPY src/ ./src/
RUN cargo build --release && \
    strip target/release/mcp-typst

# ── Etapa 2: Runtime ──────────────────────────────────────────────────────────
FROM docker.io/library/alpine:3.23

RUN apk add --update --no-cache \
    ca-certificates \
    curl \
    fontconfig \
    font-inter \
    font-montserrat \
    font-roboto \
    openssl \
    typst \
    unzip \
    && \
    adduser -S -u 1000 -D mcp

# ── Open Sans (download from Google Fonts) ────────────────────────────────────
RUN mkdir -p /usr/share/fonts/opentype && \
    curl -fsL \
      "https://github.com/google/fonts/raw/main/ofl/opensans/OpenSans%5Bwdth,wght%5D.ttf" \
      -o "/usr/share/fonts/opentype/OpenSans.ttf" && \
    curl -fsL \
      "https://github.com/google/fonts/raw/main/ofl/opensans/OpenSans-Italic%5Bwdth,wght%5D.ttf" \
      -o "/usr/share/fonts/opentype/OpenSans-Italic.ttf"

# ── Nerd Fonts ────────────────────────────────────────────────────────────────
ARG NERD_FONTS_VERSION=3.3.0
ARG NERD_FONTS="FiraCode Hack JetBrainsMono Meslo UbuntuMono"

RUN mkdir -p /usr/share/fonts/nerd-fonts && \
    for font in $NERD_FONTS; do \
      echo "Downloading $font Nerd Font v$NERD_FONTS_VERSION..."; \
      curl -fsSL \
        "https://github.com/ryanoasis/nerd-fonts/releases/download/v${NERD_FONTS_VERSION}/${font}.zip" \
        -o "/tmp/${font}.zip" && \
      unzip -qo "/tmp/${font}.zip" -d "/usr/share/fonts/nerd-fonts/${font}" && \
      rm -f "/tmp/${font}.zip"; \
    done && \
    fc-cache -f && \
    rm -rf /tmp/nerd-fonts-*

COPY --from=builder /app/target/release/mcp-typst /usr/local/bin/

USER mcp
EXPOSE 3007

CMD ["mcp-typst"]
