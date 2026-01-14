//! Inspector Web UI Module
//!
//! Provides a web-based inspector for creating test scripts visually.
//! Features:
//! - Live screen mirroring
//! - Right-click to add commands
//! - Smart selector suggestions
//! - Command playback for testing
//! - File management (create/edit YAML)

pub mod api;
pub mod screen_capture;
pub mod server;

pub use server::InspectorServer;
