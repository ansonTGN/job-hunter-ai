# --- Stage 1: Builder ---
# CAMBIO IMPORTANTE: Actualizamos de 1.75 a 1-slim-bookworm (versión actual estable)
# para que soporte el Cargo.lock versión 4.
FROM rust:1-slim-bookworm as builder

WORKDIR /usr/src/app

# Instalar dependencias de compilación
# pkg-config y libssl-dev son necesarios para compilar crates que usan red (reqwest)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copiar archivos de configuración primero para cachear dependencias
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Copiar todo el código fuente
COPY . .

# Compilar en modo release
RUN cargo build --release

# --- Stage 2: Runtime ---
FROM debian:bookworm-slim

WORKDIR /app

# Instalar dependencias de ejecución
# - ca-certificates: Para HTTPS
# - libssl3: Librería crypto
# - chromium: Necesario si activas el DynamicScraperAgent (headless chrome)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    chromium \
    && rm -rf /var/lib/apt/lists/*

# Copiar el binario compilado
COPY --from=builder /usr/src/app/target/release/job-hunter /app/job-hunter

# Copiar el frontend (La UI)
COPY --from=builder /usr/src/app/web /app/web

# Variables de entorno por defecto
ENV JOB_HUNTER_BIND=0.0.0.0
# PORT será sobreescrito por Render
ENV PORT=3000 
ENV JOB_HUNTER_WEB_DIR=/app/web
ENV RUST_LOG=info

# Render espera que escuchemos en el puerto definido por $PORT
CMD ["./job-hunter"]