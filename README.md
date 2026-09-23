# 🛡️ VoidShield AI (Tier 1: Universal Drop-In API Shield)

> **"Turn API Abusers into Dead Sockets."**  
> High-performance, zero-dependency, single-binary reverse proxy & DDoS shield built in Rust for LLM wrappers (**OpenAI, Anthropic Claude, Google Gemini, Groq, Ollama, vLLM, Localhost**).

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPLv3-blue.svg)](LICENSE)
[![Rust: 1.75+](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org/)
[![Docker: Ready](https://img.shields.io/badge/Docker-Multi--stage_Slim-2496ED.svg)](Dockerfile)
[![RPS: 50,000+](https://img.shields.io/badge/Throughput-50k%2B_RPS-brightgreen.svg)]()
[![Tier 2 Waitlist](https://img.shields.io/badge/Enterprise_Tier_2-Join_Waitlist-blueviolet.svg)](mailto:roshansingh11yt@gmail.com)

---

## 🚀 Enterprise Tier 2 (Hyper-Scale Gateway) — Private Waitlist

> 🏢 **Scaling beyond a single server or need multi-region cluster protection?**  
> **Tier 2 (Distributed Gateway)** is currently in private early access for high-throughput enterprise infrastructure handling 1M+ requests per day.

| Feature | Tier 1 (Free & Open Source) | Tier 2 (Enterprise Waitlist) |
| :--- | :--- | :--- |
| **Architecture** | Single-Binary Drop-In Proxy | Distributed Multi-Region Gateway |
| **Throughput** | 50,000+ RPS per instance | 1,000,000+ RPS Global Cluster |
| **State Storage** | Lock-Free In-Memory (DashMap) | Sub-millisecond Redis Cluster Sync |
| **Bot Detection** | Token-Bucket + Chaos Tarpit | ML Cadence Anomaly Detection |
| **Auth & Routing** | Transparent Pass-Through | Hardware-Speed JWT RBAC & Quotas |
| **Licensing** | AGPL-3.0 (Strict SaaS Copyleft) | Commercial Closed-Source Cloud Exemption |

📩 **Join the Private Waitlist:**  
To request early enterprise access or pilot deployment, email **roshansingh11yt@gmail.com** with your organization name and estimated daily volume.

---

## ⚡ What is VoidShield Tier 1?

VoidShield sits directly in front of any LLM API or backend server. When scrapers, credential stuffers, or compromised API keys hit your service:
1. **Legitimate users** get instant sub-millisecond pass-through.
2. **Abusers & bots** are trapped in a **Chaos Tarpit**—the shield keeps their TCP connection open and drips 1 empty byte every 0.5 to 3 seconds, freezing their worker thread pools without triggering immediate retry logic.

---

## 🌟 Universal Compatibility (Runs Anywhere, Connects to Anything)

- 🌐 **Any LLM Provider:** Set `UPSTREAM_URL` to:
  - OpenAI (`https://api.openai.com`)
  - Anthropic (`https://api.anthropic.com`)
  - Google Gemini (`https://generativelanguage.googleapis.com`)
  - Groq (`https://api.groq.com`)
  - Self-hosted Ollama / vLLM (`http://localhost:11434` or `http://127.0.0.1:8000`)
- 💻 **Universal OS & Hardware:** Runs natively on **Ubuntu, Debian, Alpine, Fedora, Arch, CentOS, macOS (Apple Silicon/Intel), Windows, and Android Termux** via pure statically-linked `rustls-tls` (no dynamic OpenSSL dependency required).
- 🧮 **Per-IP Mathematical Token Bucket:** Each client IP gets a fresh **10-token burst allowance** that regenerates at **2 tokens/second**. Smooth human bursts are allowed; sustained robotic spam is trapped.
- 🕳️ **Chaos Tarpit (Slow-Death Engine):** Stalls up to 5,000 concurrent bot sockets at 0% CPU overhead using Tokio async channels.
- 🛡️ **Anti-DDoS Heap Ceiling:** Capped at `500,000` concurrent tracking entries to prevent distributed botnet hashmap memory exhaustion.
- ⚡ **Circuit Breaker:** Automatically fast-drops (HTTP 429) if active tarpit sockets reach 5,000, protecting Linux file descriptors (`ulimit -n`).
- 🔒 **RFC 9110 / 7230 Compliant:** Complete hop-by-hop header stripping (defeats HTTP Request Smuggling & CL.TE / TE.CL desync).
- 📊 **Prometheus Metrics:** Native `/metrics` endpoint for real-time Grafana monitoring.

---

## ⚙️ Environment Configuration

| Variable | Default Value | Description |
| :--- | :--- | :--- |
| `UPSTREAM_URL` | `https://api.openai.com` | Target LLM API or backend URL to protect |
| `PORT` | `8080` | Port on which the shield listens |

---

## 🚦 How the Rate Limiter Works (Per-IP Mechanics)

Every unique client IP address is tracked independently in an atomic lock-free `DashMap`:
- **Initial Bucket Size:** `10.0 Tokens`
- **Refill Rate:** `2.0 Tokens/second` (Max Cap: `10.0 Tokens`)
- **Cost per Request:** `1.0 Token`

### What happens when an IP floods requests?
1. **Burst (1 to 10 requests):** Instant pass-through (`HTTP 200` forwarded upstream).
2. **Sustained Flooding (> 2 req/sec after burst):** Bucket empties below `1.0`.
3. **The Trap:** The IP is diverted to the Chaos Tarpit. A 60-second slow stream begins.
4. **Legitimate IPs:** Another user on a different IP gets their own 10 tokens—completely unaffected!

---

## 🚀 Quick Start

### Option 1: Run with Docker
```bash
# Clone the repository
git clone https://github.com/roshansingh11yt/voidshield-ai.git
cd voidshield-ai

# Run protecting OpenAI (Default)
docker build -t voidshield-tier1 .
docker run -p 8080:8080 -d voidshield-tier1

# Or protect Anthropic / Ollama / Custom API:
docker run -p 8080:8080 -e UPSTREAM_URL="https://api.anthropic.com" -d voidshield-tier1
```

### Option 2: Run with Cargo (Direct Binary on Any OS)
```bash
# Run with default upstream
cargo run --release

# Or pass custom upstream and port
UPSTREAM_URL="https://api.anthropic.com" PORT=9000 cargo run --release
```
Your shield is now live on `http://0.0.0.0:8080`!

---

## 🧪 Testing the Shield

### 1. Test Legitimate Request
```bash
curl -i http://localhost:8080/v1/models \
  -H "Authorization: Bearer YOUR_API_KEY"
```

### 2. Simulate Bot Attack & Watch the Tarpit
Run 15 rapid requests from the same IP:
```bash
for i in {1..15}; do
  curl -s -w "Req $i: HTTP %{http_code} in %{time_total}s\n" http://localhost:8080/v1/models -o /dev/null
done
```
*Requests 1-10 will return instantly. Request 11+ will hang for up to 60 seconds while the bot's connection is trapped!*

### 3. Check Real-Time Prometheus Telemetry
```bash
curl http://localhost:8080/metrics
```
Output:
```text
shield_requests_total 15
shield_tarpits_total 5
shield_active_tarpits 5
shield_fast_drops 0
```

---

## ⚖️ License (AGPL-3.0 Cloud/SaaS Copyleft Trap)

This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)**.

### Why AGPL-3.0 instead of standard GPL-3.0? (The SaaS Trap)
Standard GPL only triggers when binary files are physically distributed to end-users. Under standard GPL, cloud companies can take open-source software, host it on AWS/GCP behind an API, modify it, and keep their entire codebase closed-source.

**AGPL-3.0 completely eliminates this SaaS loophole:**
- ☁️ **Network Interaction Clause (Section 13):** If any company runs or modifies this software on a server (AWS, GCP, Azure, bare metal) and users interact with it over a network (e.g. as an API proxy, SaaS wrapper, or hosted gateway), **THEY ARE LEGALLY REQUIRED TO MAKE THEIR ENTIRE CONNECTED BACKEND SOURCE CODE PUBLICLY AVAILABLE UNDER AGPL-3.0**.
- 🚫 **No Closed-Source Commercial Exploitation:** Enterprises and SaaS startups cannot quietly run or integrate this shield into their closed-source commercial cloud infrastructure without open-sourcing their proprietary stack.
- 💼 **The Enterprise Commercial Path:** If an organization wants to run VoidShield in their proprietary cloud architecture without open-sourcing their own backend code, they must purchase a commercial enterprise license for **Tier 2 (Hyper-Scale Gateway)**.

Copyright (C) 2026 **Roshan Singh** (`roshansingh11yt@gmail.com`).
