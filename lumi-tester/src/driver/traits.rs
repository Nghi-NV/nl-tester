use crate::parser::types::{Orientation, SpeedMode};
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

/// Element selector for UI elements
#[derive(Debug, Clone)]
pub enum Selector {
    /// Select by visible text with index and exact match flag
    /// (text, index, exact) - if exact=false, use case-insensitive fallback
    Text(String, usize, bool),
    /// Select by Regex pattern on text with index
    TextRegex(String, usize),
    /// Select by resource ID with index
    Id(String, usize),
    /// Select by Regex pattern on resource ID with index
    IdRegex(String, usize),
    /// Select by class/element type with index
    Type(String, usize),
    /// Select by point coordinates
    Point { x: i32, y: i32 },
    /// Select by accessibility ID
    #[allow(dead_code)]
    AccessibilityId(String),
    /// Select by XPath
    XPath(String),
    /// Select by CSS selector (Web only)
    Css(String),
    /// Select by Image template with optional region constraint
    Image {
        path: String,
        /// Optional region to search in: top-left, top-right, bottom-left, bottom-right, etc.
        region: Option<String>,
    },
    /// Select by placeholder text with index
    Placeholder(String, usize),
    /// Select by role with index
    Role(String, usize),
    /// Select by accessibility description/content-desc with index
    Description(String, usize),
    /// Select by Regex pattern on description with index
    DescriptionRegex(String, usize),
    /// Select by scrollable container index and item index
    ScrollableItem {
        scrollable_index: usize,
        item_index: Option<usize>,
    },
    /// Select scrollable container by index
    Scrollable(usize),
    /// Select any clickable element (used as default target for relative-only selectors)
    AnyClickable(usize),
    /// Select relative to another element
    Relative {
        target: Box<Selector>,
        anchor: Box<Selector>,
        direction: RelativeDirection,
        max_dist: Option<u32>,
    },
    /// Select parent containing a child
    HasChild {
        parent: Box<Selector>,
        child: Box<Selector>,
    },
    /// Select by OCR text recognition from screenshot
    /// (text_or_regex, index, is_regex, region)
    OCR(String, usize, bool, Option<String>),
}

/// Direction for relative selection
#[derive(Debug, Clone, Copy)]
pub enum RelativeDirection {
    LeftOf,
    RightOf,
    Above,
    Below,
    Near,
}

/// Swipe direction
#[derive(Debug, Clone, Copy)]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Platform-agnostic driver interface
///
/// This trait defines all the operations that a platform driver must implement
/// to support automated testing. It abstracts away the platform-specific details
/// so that test flows can be written once and run on any supported platform.
#[async_trait]
pub trait PlatformDriver: Send + Sync {
    /// Get the platform name (e.g., "android", "ios", "web")
    #[allow(dead_code)]
    fn platform_name(&self) -> &str;

    /// Get the device serial or ID
    fn device_serial(&self) -> Option<String>;

    /// Launch an application
    ///
    /// # Arguments
    /// * `app_id` - The application identifier (package name for Android, bundle ID for iOS)
    /// * `clear_state` - If true, clear the app's data before launching
    async fn launch_app(&self, app_id: &str, clear_state: bool) -> Result<()>;

    /// Stop an application
    async fn stop_app(&self, app_id: &str) -> Result<()>;

    /// Tap on an element or coordinate
    async fn tap(&self, selector: &Selector) -> Result<()>;

    /// Long press on an element
    ///
    /// # Arguments
    /// * `selector` - The element to press
    /// * `duration_ms` - How long to hold the press in milliseconds
    async fn long_press(&self, selector: &Selector, duration_ms: u64) -> Result<()>;

    /// Double tap on an element
    async fn double_tap(&self, selector: &Selector) -> Result<()>;

    /// Right click on an element
    async fn right_click(&self, selector: &Selector) -> Result<()>;

    /// Input text at the current focus
    async fn input_text(&self, text: &str, unicode: bool) -> Result<()>;

    /// Erase text at the current focus
    ///
    /// # Arguments
    /// * `char_count` - Number of characters to erase. If None, erase all.
    async fn erase_text(&self, char_count: Option<u32>) -> Result<()>;

    /// Hide the on-screen keyboard
    async fn hide_keyboard(&self) -> Result<()>;

    /// Swipe in a direction
    ///
    /// # Arguments
    /// * `direction` - The direction to swipe
    /// * `duration_ms` - Optional swipe duration in milliseconds
    /// * `from` - Optional selector to start swipe from
    async fn swipe(
        &self,
        direction: SwipeDirection,
        duration_ms: Option<u64>,
        from: Option<Selector>,
    ) -> Result<()>;

    /// Scroll until an element becomes visible
    ///
    /// # Arguments
    /// * `selector` - The element to find
    /// * `max_scrolls` - Maximum number of scroll attempts
    ///
    /// # Returns
    /// True if the element was found, false otherwise
    async fn scroll_until_visible(
        &self,
        selector: &Selector,
        max_scrolls: u32,
        direction: Option<SwipeDirection>,
        from: Option<Selector>,
    ) -> Result<bool>;

    /// Check if an element is currently visible
    async fn is_visible(&self, selector: &Selector) -> Result<bool>;

    /// Wait for an element to become visible
    ///
    /// # Arguments
    /// * `selector` - The element to wait for
    /// * `timeout_ms` - How long to wait in milliseconds
    ///
    /// # Returns
    /// True if the element became visible, false if timeout
    async fn wait_for_element(&self, selector: &Selector, timeout_ms: u64) -> Result<bool>;

    /// Wait for an element to disappear
    async fn wait_for_absence(&self, selector: &Selector, timeout_ms: u64) -> Result<bool>;

    /// Get the text content of an element
    ///
    /// # Arguments
    /// * `selector` - The element to get text from
    ///
    /// # Returns
    /// The text content of the element, or empty string if not found
    async fn get_element_text(&self, selector: &Selector) -> Result<String>;

    /// Open a Deep Link or URL
    async fn open_link(&self, url: &str, app_id: Option<&str>) -> Result<()>;

    /// Compare current screen with a reference image
    async fn compare_screenshot(
        &self,
        reference_path: &Path,
        tolerance_percent: f64,
    ) -> Result<f64>;

    /// Take a screenshot
    ///
    /// # Arguments
    /// * `path` - Where to save the screenshot
    async fn take_screenshot(&self, path: &str) -> Result<()>;

    /// Start recording the screen
    ///
    /// # Arguments
    /// * `path` - Where to save the recording
    async fn start_recording(&self, path: &str) -> Result<()>;

    /// Stop recording the screen
    async fn stop_recording(&self) -> Result<()>;

    /// Press the back button
    async fn back(&self) -> Result<()>;

    /// Press the home button
    async fn home(&self) -> Result<()>;

    /// Get the screen size
    ///
    /// # Returns
    /// Tuple of (width, height) in pixels
    #[allow(dead_code)]
    async fn get_screen_size(&self) -> Result<(u32, u32)>;

    /// Get the current UI hierarchy as XML or JSON
    ///
    /// This is useful for debugging and element discovery
    async fn dump_ui_hierarchy(&self) -> Result<String>;

    /// Get recent system logs (Logcat for Android)
    async fn dump_logs(&self, limit: u32) -> Result<String>;

    /// Tap on an element by class type and index (0-based)
    ///
    /// # Arguments
    /// * `element_type` - Element class type (e.g., "EditText", "Button")
    /// * `index` - 0-based index of the element
    async fn tap_by_type_index(&self, _element_type: &str, _index: u32) -> Result<()> {
        Err(anyhow::anyhow!(
            "tap_by_type_index not implemented for this platform"
        ))
    }

    /// Input text at an element by class type and index
    ///
    /// # Arguments
    /// * `element_type` - Element class type
    /// * `index` - 0-based index
    /// * `text` - Text to input
    async fn input_by_type_index(
        &self,
        _element_type: &str,
        _index: u32,
        _text: &str,
    ) -> Result<()> {
        Err(anyhow::anyhow!(
            "input_by_type_index not implemented for this platform"
        ))
    }

    /// Start mock location playback from GPS points
    ///
    /// # Arguments
    /// * `name` - Optional name for this mock instance
    /// * `points` - List of GPS coordinates to simulate
    /// * `speed_kmh` - Optional speed override in km/h (ignores timestamps)
    /// * `speed_mode` - Speed simulation mode (linear or noise)
    /// * `speed_noise` - Speed noise range in km/h
    /// * `interval_ms` - Update interval in milliseconds
    /// * `loop_route` - Whether to loop the route
    async fn start_mock_location(
        &self,
        _name: Option<String>,
        _points: Vec<crate::parser::gps::GpsPoint>,
        _speed_kmh: Option<f64>,
        _speed_mode: SpeedMode,
        _speed_noise: Option<f64>,
        _interval_ms: u64,
        _loop_route: bool,
    ) -> Result<()> {
        Err(anyhow::anyhow!(
            "start_mock_location not implemented for this platform"
        ))
    }

    /// Stop mock location playback
    async fn stop_mock_location(&self) -> Result<()> {
        Err(anyhow::anyhow!(
            "stop_mock_location not implemented for this platform"
        ))
    }

    /// Get pixel color at a specific point on screen
    ///
    /// # Arguments
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    ///
    /// # Returns
    /// RGB color as (r, g, b) tuple
    async fn get_pixel_color(&self, _x: i32, _y: i32) -> Result<(u8, u8, u8)> {
        Err(anyhow::anyhow!(
            "get_pixel_color not implemented for this platform"
        ))
    }

    /// Rotate the device screen
    ///
    /// # Arguments
    /// * `mode` - "portrait" or "landscape"
    async fn rotate_screen(&self, _mode: &str) -> Result<()> {
        Err(anyhow::anyhow!(
            "rotate_screen not implemented for this platform"
        ))
    }

    /// Press a physical key
    ///
    /// # Arguments
    /// * `key` - Key name (e.g., "volume_up", "back", "home", "enter", "power")
    async fn press_key(&self, _key: &str) -> Result<()> {
        Err(anyhow::anyhow!(
            "press_key not implemented for this platform"
        ))
    }

    /// Set app permissions
    async fn set_permissions(
        &self,
        _app_id: &str,
        _permissions: &std::collections::HashMap<String, String>,
    ) -> Result<()> {
        // Default implementation does nothing
        println!("Warning: set_permissions not implemented for this platform");
        Ok(())
    }

    /// Push a file to the device
    async fn push_file(&self, _local_path: &str, _remote_path: &str) -> Result<()> {
        Err(anyhow::anyhow!(
            "push_file not implemented for this platform"
        ))
    }

    /// Pull a file from the device
    async fn pull_file(&self, _remote_path: &str, _local_path: &str) -> Result<()> {
        Err(anyhow::anyhow!(
            "pull_file not implemented for this platform"
        ))
    }

    /// Clear application data
    async fn clear_app_data(&self, _app_id: &str) -> Result<()> {
        Err(anyhow::anyhow!(
            "clear_app_data not implemented for this platform"
        ))
    }

    /// Set clipboard content
    async fn set_clipboard(&self, _text: &str) -> Result<()> {
        Err(anyhow::anyhow!(
            "set_clipboard not implemented for this platform"
        ))
    }

    /// Get clipboard content
    async fn get_clipboard(&self) -> Result<String> {
        Err(anyhow::anyhow!(
            "get_clipboard not implemented for this platform"
        ))
    }

    /// Clear iOS Simulator Keychain (iOS only)
    ///
    /// This clears all keychain items for the simulator.
    /// Only works on iOS Simulator, not physical devices.
    async fn clear_keychain(&self) -> Result<()> {
        // Default: do nothing (Android/Web don't have keychain concept)
        Ok(())
    }

    // New Commands

    /// Set network connection state
    async fn set_network_connection(&self, _wifi: Option<bool>, _data: Option<bool>) -> Result<()> {
        Err(anyhow::anyhow!("set_network_connection not implemented"))
    }

    /// Toggle airplane mode
    async fn toggle_airplane_mode(&self) -> Result<()> {
        Err(anyhow::anyhow!("toggle_airplane_mode not implemented"))
    }

    /// Open notifications panel
    async fn open_notifications(&self) -> Result<()> {
        Err(anyhow::anyhow!("open_notifications not implemented"))
    }

    /// Open quick settings panel
    async fn open_quick_settings(&self) -> Result<()> {
        Err(anyhow::anyhow!("open_quick_settings not implemented"))
    }

    /// Set device volume
    async fn set_volume(&self, _level: u8) -> Result<()> {
        Err(anyhow::anyhow!("set_volume not implemented"))
    }

    /// Lock the device
    async fn lock_device(&self) -> Result<()> {
        Err(anyhow::anyhow!("lock_device not implemented"))
    }

    /// Unlock the device
    async fn unlock_device(&self) -> Result<()> {
        Err(anyhow::anyhow!("unlock_device not implemented"))
    }

    /// Install an application
    async fn install_app(&self, _path: &str) -> Result<()> {
        Err(anyhow::anyhow!("install_app not implemented"))
    }

    /// Uninstall an application
    async fn uninstall_app(&self, _app_id: &str) -> Result<()> {
        Err(anyhow::anyhow!("uninstall_app not implemented"))
    }

    /// Send app to background and resume
    async fn background_app(&self, _app_id: Option<&str>, _duration_ms: u64) -> Result<()> {
        Err(anyhow::anyhow!("background_app not implemented"))
    }

    /// Set device orientation
    async fn set_orientation(&self, _mode: Orientation) -> Result<()> {
        Err(anyhow::anyhow!("set_orientation not implemented"))
    }

    /// Wait for device to reach a specific geographic location
    async fn wait_for_location(
        &self,
        _name: Option<String>,
        _lat: f64,
        _lon: f64,
        _tolerance: f64,
        _timeout: u64,
    ) -> Result<()> {
        Err(anyhow::anyhow!("wait_for_location not implemented"))
    }

    /// Wait for mock location playback to complete
    async fn wait_for_mock_completion(
        &self,
        _name: Option<String>,
        _timeout: Option<u64>,
    ) -> Result<()> {
        Err(anyhow::anyhow!("wait_for_mock_completion not implemented"))
    }

    /// Control a running mock location instance
    async fn control_mock_location(
        &self,
        _name: Option<String>,
        _speed: Option<f64>,
        _speed_mode: Option<SpeedMode>,
        _speed_noise: Option<f64>,
        _pause: Option<bool>,
        _resume: Option<bool>,
    ) -> Result<()> {
        Err(anyhow::anyhow!("control_mock_location not implemented"))
    }

    // Performance & Load Testing

    /// Start collecting performance metrics
    async fn start_profiling(
        &self,
        _params: Option<crate::parser::types::StartProfilingParams>,
    ) -> Result<()> {
        Err(anyhow::anyhow!("start_profiling not implemented"))
    }

    /// Stop collecting performance metrics
    async fn stop_profiling(&self) -> Result<()> {
        Err(anyhow::anyhow!("stop_profiling not implemented"))
    }

    /// Get current performance metrics snapshot
    ///
    /// # Returns
    /// Map of metric name to value (e.g. "cpu" -> 15.5, "memory" -> 256.0)
    async fn get_performance_metrics(&self) -> Result<std::collections::HashMap<String, f64>> {
        Err(anyhow::anyhow!("get_performance_metrics not implemented"))
    }

    /// Set CPU throttling rate
    ///
    /// # Arguments
    /// * `rate` - Throttling rate (e.g., 4.0 for 4x slowdown). 1.0 means no throttling.
    async fn set_cpu_throttling(&self, _rate: f64) -> Result<()> {
        Err(anyhow::anyhow!("set_cpu_throttling not implemented"))
    }

    /// Set network conditions
    ///
    /// # Arguments
    /// * `profile` - Network profile name (e.g., "Slow 3G", "Fast 3G", "Offline", or custom JSON)
    async fn set_network_conditions(&self, _profile: &str) -> Result<()> {
        Err(anyhow::anyhow!("set_network_conditions not implemented"))
    }

    /// Select target display ID (for multi-display Android/iOS)
    async fn select_display(&self, _display_id: u32) -> Result<()> {
        Ok(()) // Default no-op
    }

    /// Auto-detect Android Auto display ID
    async fn detect_android_auto_display(&self) -> Result<Option<u32>> {
        Ok(None)
    }

    /// Set device locale for i18n testing
    ///
    /// # Arguments
    /// * `locale` - Locale code (e.g., "en-US", "vi-VN", "ja-JP")
    async fn set_locale(&self, locale: &str) -> Result<()> {
        // Default: print warning and continue (for Web and unsupported platforms)
        println!(
            "  ⚠ setLocale('{}') not supported on this platform, skipping",
            locale
        );
        Ok(())
    }

    // Audio Test Commands

    /// Play media file on device
    async fn play_media(&self, _file_path: &Path, _loop_playback: bool) -> Result<()> {
        Err(anyhow::anyhow!("play_media not implemented"))
    }

    /// Stop media playback
    async fn stop_media(&self) -> Result<()> {
        Err(anyhow::anyhow!("stop_media not implemented"))
    }

    /// Start audio capture from device
    async fn start_audio_capture(&self, _duration_ms: u64, _port: u16) -> Result<()> {
        Err(anyhow::anyhow!("start_audio_capture not implemented"))
    }

    /// Stop audio capture
    async fn stop_audio_capture(&self) -> Result<()> {
        Err(anyhow::anyhow!("stop_audio_capture not implemented"))
    }

    /// Verify audio ducking occurred
    async fn verify_audio_ducking(&self, _min_events: usize, _drop_threshold: f64) -> Result<()> {
        Err(anyhow::anyhow!("verify_audio_ducking not implemented"))
    }
}
