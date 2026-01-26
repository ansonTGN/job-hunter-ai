# 🦀 Job Hunter AI: Autonomous Multi-Agent Recruitment System

![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg?style=flat-square&logo=rust)
![Docker](https://img.shields.io/badge/docker-ready-blue?style=flat-square&logo=docker)
![AI Framework](https://img.shields.io/badge/LLM-Rig--Core-blueviolet?style=flat-square)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=flat-square)

> **Sistema Distribuido de Agentes para la Caza Inteligente de Empleo.**
> *Desarrollado como Prueba de Concepto (PoC) sobre arquitecturas agénticas en Rust.*

---

## 📋 Resumen

**Job Hunter AI** es un sistema de alto rendimiento diseñado para automatizar el descubrimiento, análisis y clasificación de oportunidades laborales. A diferencia de los scrapers tradicionales, este sistema emplea una **Arquitectura de Agentes** donde entidades independientes (Scrapers, Analyzers, Enrichers) colaboran asíncronamente.

Esta versión introduce el **Recursive Logic Module (RLM)**, permitiendo a la IA "pensar" y buscar evidencias en el HTML antes de emitir un veredicto, y soporte completo para **Docker**.

## ✨ Novedades (v0.3.0)

*   🐳 **Docker Support:** Despliegue en un solo click con `docker-compose` (incluye Ollama).
*   🧠 **Recursive Logic Module (RLM):** El agente analizador ejecuta un bucle de razonamiento (Chain of Thought) para validar "Deal Breakers" y salarios ocultos.
*   📡 **15+ Fuentes de Datos:** Soporte para RemoteOK, WeWorkRemotely, Jobspresso, VueJobs, GolangProjects, Python.org, y más.
*   🛡️ **Supervisor Pattern:** El orquestador ahora captura pánicos en los agentes, evitando que un fallo en un scraper tumbe todo el sistema.
*   💾 **Exportación CSV:** Descarga los resultados analizados directamente desde la UI.

---

## ⚙️ Arquitectura Técnica

### 1. El Core (Rust & Tokio)
*   **Runtime:** `tokio` para I/O asíncrono y `tokio::sync::broadcast` para telemetría WebSocket.
*   **Mensajería:** Canales MPSC fuertemente tipados.
*   **Serialización:** Uso de `rkyv` para paso de mensajes Zero-Copy en rutas críticas.

### 2. Agentes Inteligentes
*   **Scrapers:** Una flota de agentes ligeros que consumen APIs (JSON) y parsean HTML/RSS. Incluye Rate Limiting y retardos anti-bot configurables.
*   **Analyzer (The Brain):** Utiliza LLMs (Ollama, OpenAI, Anthropic).
    *   *Modo RLM:* El agente decide: *"¿Tengo el salario? No. -> Acción: Buscar 'salary' en el HTML"*.
    *   *Safety:* Control de presupuesto para evitar costes excesivos en APIs de pago.
*   **Enricher:** Normaliza datos y formatea la salida.

### 3. Interfaz Reactiva
*   **Backend:** API REST v1 (`Axum`) + WebSockets para streaming de logs y resultados.
*   **Frontend:** Vanilla JS + CSS moderno (sin build steps). Visualización de "Match Score" y resaltado de skills.

---

## 🚀 Instalación y Uso

### Opción A: Docker (Recomendada) 🐳

Ideal para tener todo el stack (App + Ollama) listo en minutos.

1.  **Clonar el repositorio:**
    ```bash
    git clone https://github.com/tu-usuario/job-hunter-ai.git
    cd job-hunter-ai
    ```

2.  **Arrancar servicios:**
    ```bash
    docker-compose up --build -d
    ```

3.  **Configuración Inicial:**
    *   Abre `http://localhost:3000`.
    *   Ve a la configuración de IA.
    *   **Importante:** Si usas Ollama dentro de Docker, el endpoint es `http://ollama:11434` (no localhost).
    *   Descarga un modelo si es la primera vez: `docker exec -it job-hunter-ollama ollama pull llama3`.

### Opción B: Ejecución Local (Rust) 🦀

Para desarrollo y máxima velocidad.

1.  **Requisitos:** Rust 1.75+, Ollama (opcional).
2.  **Ejecutar:**
    ```bash
    cargo run --release
    ```
3.  **UI:** Accede a `http://localhost:3000`.

---

## 🧩 Fuentes Soportadas

El sistema incluye scrapers especializados para:

| Fuente | Tipo | Estado |
|--------|------|--------|
| **RemoteOK** | API | ✅ Activo |
| **WeWorkRemotely** | RSS/HTML | ✅ Activo |
| **Arbeitnow** | API | ✅ Activo |
| **Himalayas** | API | ✅ Activo |
| **Jobspresso** | HTML Scraper | ✅ Activo |
| **Remotive** | API | ✅ Activo |
| **VueJobs** | API | ✅ Activo |
| **GolangProjects** | API | ✅ Activo |
| **Python.org** | RSS | ✅ Activo |
| *... y 6 más* | JSON/RSS | ✅ Activo |

---

## 🧠 Configuración de IA

El sistema soporta "Provider Agnosticism". Puedes cambiar en caliente entre:

1.  **Local (Ollama):** Coste cero, privacidad total. Recomendado: `llama3` o `mistral`.
2.  **OpenAI (GPT-4o):** Máxima precisión para análisis profundos.
3.  **Anthropic (Claude 3.5 Sonnet):** Excelente equilibrio para razonamiento y extracción de contexto largo.

> **Nota:** Puedes configurar las API Keys desde la UI (se guardan en memoria o `.env` según prefieras).

---

## 📂 Estructura del Proyecto

```bash
.
├── crates/
│   ├── core/           # Tipos compartidos (Domain)
│   ├── agents/         # Lógica de Scrapers y Analyzer (RLM)
│   ├── orchestrator/   # Supervisor y enrutamiento de mensajes
│   └── ui/             # Helpers de consola
├── src/
│   ├── adapters/       # API REST y WebSockets (Axum)
│   └── main.rs         # Entrypoint
├── web/                # Frontend (HTML/JS/CSS)
├── Dockerfile          # Multi-stage build
└── docker-compose.yml  # Orquestación de contenedores
```

---

## 🔮 Roadmap

*   [ ] **Base de Datos:** Persistencia en PostgreSQL/SQLite (actualmente en memoria).
*   [ ] **Notificaciones:** Integración con Telegram/Discord.
*   [ ] **Headless Browser:** Reactivar el `DynamicScraperAgent` (Chrome) para sitios SPA complejos.

---

## 📄 Licencia

Distribuido bajo la **MIT License**.

---
*Developed with Rust & ❤️*
--- END OF FILE README.md ---