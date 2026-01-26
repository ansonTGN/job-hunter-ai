# --- Stage 1: Builder ---
FROM rust:1.75-slim-bookworm as builder

WORKDIR /usr/src/app

# Instalar dependencias de compilación (OpenSSL es crítico para reqwest)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copiar todo el código fuente (respetando .dockerignore)
COPY . .

# Compilar en modo release
# Esto puede tardar unos minutos dependiendo de las dependencias
RUN cargo build --release

# --- Stage 2: Runtime ---
FROM debian:bookworm-slim

WORKDIR /app

# Instalar dependencias de ejecución
# - ca-certificates: Para hacer peticiones HTTPS (scrapers)
# - libssl3: Librería de encriptación requerida por el binario compilado
# - chromium: (Opcional) Si descomentas el DynamicScraperAgent en el futuro, necesitarás esto.
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copiar el binario desde la etapa de compilación
COPY --from=builder /usr/src/app/target/release/job-hunter /app/job-hunter

# Copiar la carpeta del frontend (indispensable para la UI)
COPY --from=builder /usr/src/app/web /app/web

# Variables de entorno por defecto
ENV JOB_HUNTER_BIND=0.0.0.0
ENV JOB_HUNTER_PORT=3000
ENV JOB_HUNTER_WEB_DIR=/app/web
ENV RUST_LOG=info

# Exponer el puerto
EXPOSE 3000

# Ejecutar la aplicación
CMD ["./job-hunter"]