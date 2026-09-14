//! Inspector Web Server
//!
//! HTTP + WebSocket server for the Inspector UI.

use anyhow::Result;
use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use super::api::{self, AppState};
use super::screen_capture::ScreenCapture;

/// Inspector server configuration
pub struct InspectorConfig {
    pub port: u16,
    pub platform: String,
    pub device_serial: Option<String>,
    pub output_file: Option<std::path::PathBuf>,
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self {
            port: 9333,
            platform: "android".to_string(),
            device_serial: None,
            output_file: None,
        }
    }
}

/// Main inspector server
pub struct InspectorServer {
    config: InspectorConfig,
}

impl InspectorServer {
    /// Create a new inspector server
    pub fn new(config: InspectorConfig) -> Self {
        Self { config }
    }

    /// Start the server
    pub async fn start(&self) -> Result<()> {
        ensure_port_free(self.config.port).await;

        // Initialize screen capture
        let screen_capture =
            ScreenCapture::new(&self.config.platform, self.config.device_serial.as_deref()).await?;

        let state = Arc::new(AppState {
            screen_capture,
            yaml_file: std::sync::Mutex::new(self.config.output_file.clone()),
            device_serial: self.config.device_serial.clone(),
            current_target_app: std::sync::Mutex::new(self.config.device_serial.clone()),
            cached_hierarchy: std::sync::Mutex::new(None),
        });

        // Build router
        //
        // `.allow_private_network(true)` matters specifically for newer
        // Chromium-based hosts (confirmed live: Antigravity IDE, likely a newer
        // Electron/Chromium than upstream VS Code) that enforce Private Network
        // Access - a CORS preflight requiring the server to explicitly opt in
        // before a page can fetch a private-network address like `localhost`.
        // `.permissive()` alone doesn't set this, so `curl`/direct requests to
        // this server worked fine while every `fetch()` call from inside the
        // Inspector's embedded webview iframe failed outright with "Failed to
        // fetch" - the initial iframe navigation isn't gated by PNA, only the
        // JS-initiated subresource requests afterward, which is exactly the
        // failure pattern this surfaced as.
        let app = Router::new()
            .route("/", get(serve_index))
            .merge(api::api_router())
            .layer(CorsLayer::permissive().allow_private_network(true))
            .with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));

        println!("\n🔍 Inspector started!");
        println!("   Open: http://localhost:{}", self.config.port);
        println!("   Platform: {}", self.config.platform);
        if let Some(ref serial) = self.config.device_serial {
            println!("   Device: {}", serial);
        }
        if let Some(ref file) = self.config.output_file {
            println!("   Output: {}", file.display());
        }
        println!("\n   Press Ctrl+C to stop.\n");

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app.into_make_service()).await?;

        Ok(())
    }
}

/// Before binding, check whether `port` is already held by a *stale lumi-tester process*
/// (most commonly: a previous `inspect` invocation left running in another terminal, or
/// orphaned after upgrading to a new CLI version - the exact "port conflict with the
/// previous version" failure mode reported live: device selection succeeded but every
/// snapshot request failed until the user manually found and killed the old process) and,
/// if so, kill it so the upcoming `bind()` succeeds without the user having to do that by
/// hand. Deliberately conservative: a port held by anything that doesn't look like our own
/// binary is left alone - `bind()` below will then fail with its own clear OS error rather
/// than this function guessing wrong and killing an unrelated process. Best-effort only
/// (requires `lsof`/`ps`, present on macOS/Linux; silently does nothing where they aren't,
/// e.g. Windows - `bind()` still surfaces the normal "address in use" error there).
async fn ensure_port_free(port: u16) {
    if tokio::net::TcpListener::bind(("0.0.0.0", port)).await.is_ok() {
        // Nothing was listening - the probe listener drops (releasing the port) as this
        // function returns, before the caller's own bind() runs.
        return;
    }

    let lsof = tokio::process::Command::new("lsof")
        .args(["-ti", &format!("tcp:{}", port), "-sTCP:LISTEN"])
        .output()
        .await;
    let Ok(lsof) = lsof else { return }; // lsof unavailable (e.g. Windows) - nothing to do.
    if !lsof.status.success() {
        return;
    }

    let pids: Vec<u32> = String::from_utf8_lossy(&lsof.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .collect();
    if pids.is_empty() {
        return;
    }

    let mut killed_any = false;
    let mut left_unrelated = false;
    for pid in pids {
        let ps = tokio::process::Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "command="])
            .output()
            .await;
        let cmdline = ps.ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string());
        let looks_like_us = cmdline
            .as_deref()
            .map(|c| c.contains("lumi-tester") || c.contains("lumi_tester"))
            .unwrap_or(false);

        if looks_like_us {
            eprintln!(
                "  🔧 Port {} is held by a stale lumi-tester process (pid {}) - stopping it...",
                port, pid
            );
            let _ = tokio::process::Command::new("kill")
                .args(["-9", &pid.to_string()])
                .output()
                .await;
            killed_any = true;
        } else {
            left_unrelated = true;
        }
    }

    if left_unrelated {
        eprintln!(
            "  ⚠️ Port {} is held by another process that isn't lumi-tester - leaving it alone. \
             If startup fails below, free the port manually or pass --port to use a different one.",
            port
        );
    }
    if killed_any {
        // Give the OS a moment to actually release the socket after kill -9.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
}

/// Serve the main HTML page with inlined CSS/JS
async fn serve_index() -> impl IntoResponse {
    let mut html = include_str!("ui/inspector.html").to_string();
    let css = include_str!("ui/style.css");
    let js = include_str!("ui/script.js");

    // Inline assets
    html = html.replace("</head>", &format!("<style>{}</style></head>", css));
    html = html.replace("</body>", &format!("<script>{}</script></body>", js));

    Html(html)
}
