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
        // Initialize screen capture
        let screen_capture =
            ScreenCapture::new(&self.config.platform, self.config.device_serial.as_deref()).await?;

        let state = Arc::new(AppState {
            screen_capture,
            yaml_file: std::sync::Mutex::new(self.config.output_file.clone()),
            device_serial: self.config.device_serial.clone(),
            cached_hierarchy: std::sync::Mutex::new(None),
        });

        // Build router
        let app = Router::new()
            .route("/", get(serve_index))
            .merge(api::api_router())
            .layer(CorsLayer::permissive())
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
