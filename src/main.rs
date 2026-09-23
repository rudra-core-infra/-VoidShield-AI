use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode},
    routing::{any, get},
    Router,
};
use dashmap::DashMap;
use rand::Rng;
use reqwest::Client;
use std::{
    env,
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{signal, time::sleep};

// --- Telemetry Counters ---
static REQS_TOTAL: AtomicUsize = AtomicUsize::new(0);
static TARPITS_TOTAL: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_TARPITS: AtomicUsize = AtomicUsize::new(0);
static FAST_DROPS: AtomicUsize = AtomicUsize::new(0);

// --- Mathematical Limits & Kernel Protections ---
// Scaled to 512 so active tarpit sockets never exhaust Linux default ulimit -n (1024)
const MAX_ACTIVE_TARPITS: usize = 512;
const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // Strict 10MB Anti-OOM memory buffer cap
const MAX_TRACKED_ENTRIES: usize = 500_000;    // Hard ceiling on DashMap allocations (Heap exhaustion shield)

// --- Precision Clock-Skew Hardened Token Bucket ---
struct TokenBucket {
    tokens: f64,
    last: Instant,
}

impl TokenBucket {
    fn consume(&mut self, rate: f64, cap: f64) -> bool {
        let now = Instant::now();
        let elapsed = now.checked_duration_since(self.last).unwrap_or_default().as_secs_f64();
        self.tokens = (self.tokens + elapsed * rate).min(cap);
        self.last = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

// --- Shield Application State ---
struct AppState {
    limits: DashMap<String, TokenBucket>,
    upstream: String,
    client: Client,
}

// --- RFC 9110 / RFC 7230 / HTTP/2 Hop-by-Hop & Framing Sanitizer ---
fn is_hop_by_hop_or_framing(name: &HeaderName) -> bool {
    let s = name.as_str();
    s.eq_ignore_ascii_case("host")
        || s.eq_ignore_ascii_case("connection")
        || s.eq_ignore_ascii_case("keep-alive")
        || s.eq_ignore_ascii_case("proxy-authenticate")
        || s.eq_ignore_ascii_case("proxy-authorization")
        || s.eq_ignore_ascii_case("te")
        || s.eq_ignore_ascii_case("trailer")
        || s.eq_ignore_ascii_case("transfer-encoding")
        || s.eq_ignore_ascii_case("upgrade")
        || s.eq_ignore_ascii_case("content-length")
}

// --- Hardened IP Resolver (Defeats Header Spoofing via Peer Socket Fallback) ---
fn extract_client_ip(h: &HeaderMap, peer_addr: SocketAddr) -> String {
    // Only trust reverse proxy headers if explicitly enabled via TRUST_PROXY env
    let trust_proxy = env::var("TRUST_PROXY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if trust_proxy {
        let raw = h
            .get("cf-connecting-ip")
            .or_else(|| h.get("x-real-ip"))
            .or_else(|| h.get("x-forwarded-for"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim());

        if let Some(candidate) = raw {
            if let Ok(parsed) = candidate.parse::<std::net::IpAddr>() {
                return parsed.to_string();
            }
        }
    }

    // Default: Hardware TCP Socket IP (Impossible for remote clients to forge or spoof)
    peer_addr.ip().to_string()
}

// --- Production Lifecycle: Signal Trapping (SIGINT / SIGTERM) ---
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("Failed to bind Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to bind SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => println!("🛡️ [SHIELD] Received SIGINT (Ctrl+C). Initiating graceful shutdown..."),
        _ = terminate => println!("🛡️ [SHIELD] Received SIGTERM (Docker/K8s). Initiating graceful shutdown..."),
    }
}

#[tokio::main]
async fn main() {
    // Universal Upstream & Port Configuration (Connects with OpenAI, Anthropic, Gemini, Ollama, Localhost)
    let upstream = env::var("UPSTREAM_URL").unwrap_or_else(|_| "https://api.openai.com".to_string());
    let bind_port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_addr = format!("0.0.0.0:{bind_port}");

    let state = Arc::new(AppState {
        limits: DashMap::new(),
        upstream: upstream.clone(),
        client: Client::builder()
            .timeout(Duration::from_secs(300)) // Extended 300s timeout for long LLM reasoning streams
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(256)       // High-throughput idle pool
            .pool_idle_timeout(Duration::from_secs(60))
            .tcp_keepalive(Duration::from_secs(60))
            .tcp_nodelay(true)
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .build()
            .expect("Fatal: Failed to construct hardened upstream HTTP client"),
    });

    // Zero-Jitter Adaptive Sweeper (Bounded iterative cursor prevents shard lock spikes)
    let cleanup = state.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(60)).await;
            let expired_keys: Vec<String> = cleanup
                .limits
                .iter()
                .filter(|entry| {
                    Instant::now()
                        .checked_duration_since(entry.value().last)
                        .unwrap_or_default()
                        > Duration::from_secs(300)
                })
                .take(500)
                .map(|entry| entry.key().clone())
                .collect();

            for key in expired_keys {
                cleanup.limits.remove(&key);
            }
        }
    });

    let app = Router::new()
        .route("/metrics", get(metrics))
        .route("/*path", any(proxy))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|err| panic!("Fatal: Failed to bind on {bind_addr}: {err}"));
    
    println!("🛡️ Tier 1 Drop-in Shield [UNIVERSAL HARDENED v3.2] listening on {bind_addr}");
    println!("🔗 Forwarding upstream target: {upstream}");

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn proxy(
    State(s): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    h: HeaderMap,
    req: Request<Body>,
) -> axum::response::Response<Body> {
    REQS_TOTAL.fetch_add(1, Ordering::Relaxed);

    // Strict URI Path Normalization & Traversal Shield
    let raw_path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("/");
    if raw_path.contains("..") || raw_path.starts_with("//") {
        return axum::response::Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Body::from("Bad Request: Invalid path traversal detected"))
            .unwrap();
    }

    // Hardware-verified IP extraction defeats X-Forwarded-For injection
    let ip = extract_client_ip(&h, addr);

    // Rate Limiting Evaluation with In-Memory Allocation Guard
    let is_tarpitted = {
        if s.limits.len() >= MAX_TRACKED_ENTRIES && !s.limits.contains_key(&ip) {
            true // Anti-Memory-Exhaustion fail-safe under extreme distributed bot floods
        } else {
            let mut bucket = s.limits.entry(ip).or_insert(TokenBucket {
                tokens: 10.0,
                last: Instant::now(),
            });
            !bucket.consume(2.0, 10.0)
        }
    };

    if is_tarpitted {
        TARPITS_TOTAL.fetch_add(1, Ordering::Relaxed);

        // Kernel File-Descriptor Protection: Circuit Breaker capped to safe ulimit bounds (512)
        let current_active = ACTIVE_TARPITS.load(Ordering::Relaxed);
        if current_active >= MAX_ACTIVE_TARPITS {
            FAST_DROPS.fetch_add(1, Ordering::Relaxed);
            return axum::response::Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .header(header::RETRY_AFTER, "60")
                .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(Body::from("Rate limit exceeded (Circuit Breaker Active)"))
                .unwrap();
        }

        ACTIVE_TARPITS.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<axum::body::Bytes, std::convert::Infallible>>(1);
        let body = Body::from_stream(tokio_stream::wrappers::ReceiverStream::new(rx));

        tokio::spawn(async move {
            struct TarpitGuard;
            impl Drop for TarpitGuard {
                fn drop(&mut self) {
                    ACTIVE_TARPITS.fetch_sub(1, Ordering::Relaxed);
                }
            }
            let _guard = TarpitGuard;

            let start = Instant::now();
            let mut iterations = 0;

            // Stream bounded chunks; terminates cleanly on socket disconnect, timeout, or chunk limit
            while start.elapsed() < Duration::from_secs(60) && iterations < 30 {
                let delay = rand::thread_rng().gen_range(500..3000);
                sleep(Duration::from_millis(delay)).await;
                iterations += 1;

                if tx.send(Ok(axum::body::Bytes::from(" "))).await.is_err() {
                    break;
                }
            }
        });

        return axum::response::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
            .header("X-Content-Type-Options", "nosniff")
            .body(body)
            .unwrap();
    }

    // Path & Target URL Normalization
    let upstream_base = s.upstream.trim_end_matches('/');
    let clean_path = if raw_path.starts_with('/') { raw_path } else { "/" };
    let target_url = format!("{upstream_base}{clean_path}");

    // Target Host Derivation for Virtual Host & TLS SNI Synchronization
    let host_header = target_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("api.openai.com");

    let mut proxy_req = s.client.request(req.method().clone(), &target_url);
    if let Ok(host_val) = HeaderValue::from_str(host_header) {
        proxy_req = proxy_req.header(header::HOST, host_val);
    }

    for (k, v) in req.headers() {
        if !is_hop_by_hop_or_framing(k) {
            proxy_req = proxy_req.header(k, v);
        }
    }

    // Anti-OOM: 10MB In-Flight Memory Bounded Buffer
    let req_body = match axum::body::to_bytes(req.into_body(), MAX_BODY_SIZE).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return axum::response::Response::builder()
                .status(StatusCode::PAYLOAD_TOO_LARGE)
                .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(Body::from("Payload Too Large (Max 10MB)"))
                .unwrap();
        }
    };
    let proxy_req = proxy_req.body(req_body).build().unwrap();

    match s.client.execute(proxy_req).await {
        Ok(res) => {
            let mut builder = axum::response::Response::builder().status(res.status());
            for (k, v) in res.headers() {
                if !is_hop_by_hop_or_framing(k) {
                    builder = builder.header(k, v);
                }
            }
            builder.body(Body::from_stream(res.bytes_stream())).unwrap()
        }
        Err(err) => {
            let status = if err.is_timeout() {
                StatusCode::GATEWAY_TIMEOUT
            } else {
                StatusCode::BAD_GATEWAY
            };
            axum::response::Response::builder()
                .status(status)
                .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(Body::from(format!("Upstream Gateway Error: {err}")))
                .unwrap()
        }
    }
}

async fn metrics() -> String {
    format!(
        "shield_requests_total {}\nshield_tarpits_total {}\nshield_active_tarpits {}\nshield_fast_drops {}\n",
        REQS_TOTAL.load(Ordering::Relaxed),
        TARPITS_TOTAL.load(Ordering::Relaxed),
        ACTIVE_TARPITS.load(Ordering::Relaxed),
        FAST_DROPS.load(Ordering::Relaxed)
    )
}