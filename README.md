# 🦀 Job Hunter AI: Autonomous Multi-Agent Recruitment System

![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg?style=flat-square&logo=rust)
![Architecture](https://img.shields.io/badge/architecture-distributed__actors-blueviolet?style=flat-square)
![AI Framework](https://img.shields.io/badge/LLM-Rig--Core-blue?style=flat-square)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=flat-square)

> **Proof of Concept (PoC) for Distributed Agentic Architectures & LLM Reasoning Loops.**
> *Developed by [Angel A. Urbina](https://github.com/AngelUrbina)*

---

## 📋 Overview

**Job Hunter AI** is a high-performance, distributed system designed to automate the discovery, analysis, and ranking of job opportunities. Unlike traditional scrapers, this system employs an **Agentic Architecture** where independent entities (Scrapers, Analyzers, Enrichers) collaborate asynchronously to solve a complex task.

This project serves as a **technical playground** to test the limits of Rust in AI orchestration and to implement concepts from recent research papers regarding **Recursive Reasoning (CoT)**, **Self-Reflection**, and **Tool Use** in Large Language Models (LLMs).

## ⚠️ Disclaimer: Research Prototype

This software is a **Proof of Concept (PoC)**. Its primary goal is to evaluate the latency, cost, and accuracy trade-offs of different agentic patterns (ReAct vs. Linear Chains) in a real-world scenario. While functional, it is intended for educational and research purposes.

---

## 🔬 Theoretical Foundation & Implemented Papers

This project implements architectures and patterns derived from cutting-edge research in Generative Agents. The **Analyzer Agent** specifically utilizes a **Recursive Logic Module (RLM)** inspired by:

1.  **ReAct: Synergizing Reasoning and Acting in Language Models (Yao et al., 2023)**
    *   *Implementation:* The `AnalyzerAgent` does not simply "guess" a match score. It enters a reasoning loop where it can choose to execute tools (`search_job_text`, `read_cv_segment`) before deciding to `finalize` the answer.
    *   *Goal:* To reduce hallucination by grounding the LLM's output in retrieved context.

2.  **Reflexion: Language Agents with Verbal Reinforcement Learning (Shinn et al., 2023)**
    *   *Implementation:* The system includes a feedback mechanism where, if the LLM produces malformed JSON or insufficient evidence, the Orchestrator (or the agent itself via RLM) iterates on the prompt to self-correct before presenting the result to the user.

3.  **Communicative Agents for Software Development (ChatDev pattern)**
    *   *Implementation:* Separation of concerns into distinct roles (`Scraper` for data ingestion, `Analyzer` for cognitive processing, `Enricher` for metadata expansion) communicating via message passing, mimicking a specialized workforce.

---

## ⚙️ Technical Architecture

### 1. The Core: Rust & Tokio
The system is built on **Rust** to ensure memory safety and maximum concurrency without the overhead of a Garbage Collector.
*   **Runtime:** `tokio` for asynchronous I/O.
*   **Concurrency Model:** Actor-like pattern using `tokio::sync::mpsc` channels for inter-agent communication and `tokio::sync::broadcast` for real-time WebSocket telemetry.
*   **Serialization:** Utilizes **`rkyv`** (Zero-Copy deserialization) for internal message passing between critical paths, ensuring minimal latency when moving large data structures (like job listings) across threads.

### 2. The Orchestrator
The `Orchestrator` acts as the central nervous system. It does not process data but manages the lifecycle of the search:
1.  Spawns worker threads for selected sources.
2.  Routes messages (payloads) between agents (`Scraper` -> `Analyzer` -> `Enricher`).
3.  Handles graceful shutdowns and error propagation.
4.  Ensures system resilience (a failure in one scraper does not crash the entire pipeline).

### 3. The Analyzer Agent (The Brain)
This is where the LLM integration lives. It supports **Polymorphic Providers**:
*   **Local:** Ollama (Llama 3, Mistral) for zero-cost inference.
*   **Cloud:** OpenAI (GPT-4o) or Anthropic (Claude 3.5 Sonnet) for high-precision tasks.

**Workflow:**
1.  **Ingestion:** Receives raw HTML/JSON and the user's PDF CV.
2.  **Extraction:** Uses `lopdf` for robust PDF text extraction.
3.  **Reasoning Loop (RLM):** The agent autonomously decides:
    *   *"Do I have enough info about the salary?"* -> Action: `grep("salary")`.
    *   *"Does the CV mention Rust?"* -> Action: `grep_cv("Rust")`.
4.  **Synthesis:** Produces a structured JSON object with a `match_score` (0.0 - 1.0) and justification.

### 4. Frontend & Real-time Telemetry
*   **Backend:** `Axum` serves the REST API and upgrades connections to WebSockets.
*   **Frontend:** Vanilla JS/Vue module architecture (no build step required for ease of deployment) with reactive state management.
*   **Protocol:** Custom JSON-based protocol over WebSockets for streaming logs (`log`), job updates (`job_found`), and status changes.

---

## 📂 Project Structure

```bash
.
├── crates/
│   ├── core/           # Shared types, Traits, Rkyv structs
│   ├── agents/         # Implementation of specific agents (Scrapers, Analyzer)
│   ├── orchestrator/   # Message routing logic and lifecycle management
│   ├── llm/            # LLM Abstractions
│   └── ui/             # CLI output helpers
├── src/
│   ├── adapters/       # HTTP/WebSocket handlers (Axum)
│   └── main.rs         # Application entry point
├── web/                # Frontend assets (Vue/Vanilla JS)
└── Cargo.toml          # Workspace configuration
```

---

## 🚀 Getting Started

### Prerequisites
*   **Rust:** `1.75+`
*   **Ollama:** (Optional) If you want to run local inference.
*   **API Keys:** (Optional) OpenAI or Anthropic keys if using cloud models.

### Installation

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/AngelUrbina/job-hunter-ai.git
    cd job-hunter-ai
    ```

2.  **Configure environment:**
    Create a `.env` file (optional, UI can handle config):
    ```env
    JOB_HUNTER_PORT=3000
    ```

3.  **Run in Release Mode:**
    (Release mode is recommended for the Scraper's async performance)
    ```bash
    cargo run --release
    ```

4.  **Access the Dashboard:**
    Open `http://localhost:3000` in your browser.

---

## 🔮 Future Roadmap

*   [ ] **Vector Database Integration:** Implement `pgvector` or `Qdrant` to store job embeddings for semantic search rather than keyword matching.
*   [ ] **Multi-Modal Agents:** Allow the agents to "see" screenshots of job boards to bypass complex DOM structures (Vision-Language Models).
*   [ ] **Planner Agent:** Implement a high-level planner that can dynamically decide which sources to scrape based on the user's query intent.

---

## 📄 License

Distributed under the **MIT License**. See `LICENSE` for more information.

---

**Author:** Angel A. Urbina  
*Exploring the frontiers of Agentic AI with Rust.*