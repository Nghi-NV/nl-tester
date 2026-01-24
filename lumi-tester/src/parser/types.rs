use serde::{Deserialize, Serialize};
use serde_json;
use serde_yaml;
use std::collections::HashMap;

/// Represents a parsed test flow from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestFlow {
    #[serde(default)]
    pub app_id: Option<String>,

    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub platform: Option<Platform>,

    #[serde(default)]
    pub env: Option<HashMap<String, String>>,

    #[serde(default)]
    pub data: Option<String>,

    #[serde(default, alias = "defaultTimeout")]
    pub default_timeout_ms: Option<u64>,

    #[serde(default)]
    pub commands: Vec<TestCommand>,

    #[serde(default)]
    pub tags: Vec<String>,

    /// Speed profile for this flow: "turbo", "fast", "normal", "safe"
    #[serde(default)]
    pub speed: Option<String>,

    /// Web browser type: "Chrome", "Firefox", "Webkit"
    #[serde(default)]
    pub browser: Option<String>,

    /// Whether to close browser when test finishes (default: true)
    #[serde(default)]
    pub close_when_finish: Option<bool>,
}

/// Target platform for testing
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    #[default]
    Android,
    #[serde(alias = "android_auto")]
    AndroidAuto,
    #[serde(alias = "iOS")]
    Ios,
    Web,
}

/// Device orientation modes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Orientation {
    Portrait,
    Landscape,
    UpsideDown,
    LandscapeLeft,
    LandscapeRight,
}

// Forward declarations for new param types used in TestCommand enum
// (Full definitions are below)

/// Parameters for assertTrue command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertTrueCondition {
    pub condition: String,
    #[serde(default)]
    pub soft: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum AssertTrueParams {
    Expression(String),
    Condition(AssertTrueCondition),
}

/// Parameters for copyTextFrom command
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CopyTextFromParams {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub index: Option<usize>,
}

/// Parameters for inputRandomNumber
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RandomNumberParams {
    #[serde(default)]
    pub length: Option<u32>,
}

/// Parameters for inputRandomText
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RandomTextParams {
    #[serde(default)]
    pub length: Option<u32>,
}

/// Parameters for extendedWaitUntil (forward declaration - uses AssertParams)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtendedWaitParams {
    pub timeout: u64,
    #[serde(default)]
    pub visible: Option<Box<serde_json::Value>>,
    #[serde(default)]
    pub not_visible: Option<Box<serde_json::Value>>,
}

/// All supported test commands
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestCommand {
    // App lifecycle
    #[serde(alias = "open")]
    LaunchApp(Option<LaunchAppParamsInput>),
    StopApp,

    // Interactions
    #[serde(alias = "tap")]
    TapOn(TapParamsInput),
    #[serde(alias = "longPress")]
    LongPressOn(TapParamsInput),
    #[serde(alias = "doubleTap")]
    DoubleTapOn(TapParamsInput),
    #[serde(alias = "write")]
    InputText(InputTextParamsInput),
    EraseText(Option<EraseTextParams>),
    HideKeyboard,
    #[serde(rename = "rightClick", alias = "contextClick")]
    RightClick(TapParams),

    // Indexed interactions (by element type and index)
    TapAt(TapAtParams),
    InputAt(InputAtParams),

    // Swipe/Scroll
    SwipeLeft,
    SwipeRight,
    SwipeUp,
    SwipeDown,
    #[serde(alias = "swipe")]
    ManualScroll(Option<ScrollParams>),
    #[serde(alias = "scrollTo")]
    ScrollUntilVisible(ScrollUntilVisibleInput),

    // Assertions
    #[serde(alias = "see")]
    AssertVisible(AssertParamsInput),
    #[serde(alias = "notSee")]
    AssertNotVisible(AssertParamsInput),
    #[serde(alias = "waitUntilVisible", alias = "waitSee")]
    WaitUntilVisible(AssertParamsInput),
    #[serde(alias = "waitNotSee")]
    WaitUntilNotVisible(AssertParamsInput),

    // Control flow
    WaitForAnimationToEnd,
    #[serde(alias = "await")]
    Wait(WaitParamsInput),
    Repeat(RepeatParams),
    Retry(RetryParams),
    RunFlow(RunFlowParamsInput),

    // Variables
    SetVar(SetVarParams),
    AssertVar(AssertVarParams),

    // Media
    #[serde(alias = "openLink", alias = "deepLink")]
    OpenLink(String),
    #[serde(alias = "assertScreenshot")]
    AssertScreenshot(String),
    TakeScreenshot(ScreenshotParamsInput),
    StartRecording(RecordingParamsInput),
    StopRecording,

    // Report
    ExportReport(ReportParams),

    // Navigation
    Back,
    PressHome,

    // Advanced Features
    Generate(GenerateParams),
    HttpRequest(HttpRequestParams),
    RunScript(RunScriptParamsInput),
    Conditional(ConditionalParams),

    // Web-specific (Future)
    Navigate(NavigateParams),
    Click(ClickParams),
    Type(TypeParams),

    // GPS Mock Location
    #[serde(alias = "gps")]
    MockLocation(MockLocationParamsInput),
    StopMockLocation,
    MockLocationControl(MockLocationControlParams),

    // Visual Assertions
    #[serde(alias = "checkColor")]
    AssertColor(AssertColorParams),

    // GIF Recording
    #[serde(alias = "captureFrame")]
    CaptureGifFrame(CaptureGifFrameParamsInput),
    #[serde(alias = "createGif")]
    BuildGif(BuildGifParams),
    // GIF Auto-Capture (interval-based)
    StartGifCapture(StartGifCaptureParams),
    StopGifCapture(StopGifCaptureParams),

    // Device Control
    #[serde(alias = "rotate")]
    RotateScreen(RotationParamsInput),
    #[serde(alias = "press")]
    PressKey(String),

    // File Management
    PushFile(FileTransferParams),
    PullFile(FileTransferParams),
    ClearAppData(String), // package_id

    // Clipboard
    #[serde(alias = "setClipboard")]
    SetClipboard(String),
    #[serde(alias = "getClipboard")]
    GetClipboard(SetVarParams), // save to variable
    #[serde(alias = "assertClipboard")]
    AssertClipboard(String),

    #[serde(alias = "assert")]
    AssertTrue(AssertTrueParams),
    EvalScript(String),

    // Clipboard Operations
    CopyTextFrom(CopyTextFromParams),
    PasteText,

    // Random Input
    InputRandomEmail,
    InputRandomNumber(Option<RandomNumberParams>),
    InputRandomPersonName,
    InputRandomText(Option<RandomTextParams>),

    // Extended Wait
    ExtendedWaitUntil(ExtendedWaitParams),

    // Database
    #[serde(alias = "dbQuery")]
    DbQuery(DbQueryParams),

    // Network & Connectivity
    #[serde(alias = "setNetwork")]
    SetNetwork(NetworkParams),
    #[serde(alias = "airplaneMode")]
    ToggleAirplaneMode,

    // System Interactions
    OpenNotifications,
    OpenQuickSettings,
    SetVolume(u8),
    LockDevice,
    UnlockDevice,

    // App Management
    InstallApp(String),
    UninstallApp(String),
    BackgroundApp(BackgroundAppParams),

    // Device Orientation
    #[serde(alias = "setOrientation")]
    SetOrientation(OrientationParams),
    // Mock Location Sync
    WaitForLocation(WaitForLocationParams),
    WaitForMockCompletion(WaitForMockCompletionParams),

    // Performance & Load Testing
    #[serde(alias = "startProfiling")]
    StartProfiling(Option<StartProfilingParams>),
    #[serde(alias = "stopProfiling")]
    StopProfiling(Option<StopProfilingParams>),
    #[serde(alias = "assertPerformance")]
    AssertPerformance(AssertPerformanceParams),
    #[serde(alias = "setCpuThrottling")]
    SetCpuThrottling(f64),
    #[serde(alias = "setNetworkConditions")]
    SetNetworkConditions(String),

    #[serde(alias = "display")]
    SelectDisplay(String),

    // Locale/Language
    #[serde(alias = "locale")]
    SetLocale(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitForLocationParams {
    /// Name of the mock location instance to wait for (optional, defaults to default instance)
    #[serde(default)]
    pub name: Option<String>,

    pub lat: f64,
    pub lon: f64,
    #[serde(default = "default_tolerance_meters")]
    pub tolerance: f64,
    #[serde(default = "default_wait_ms")]
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitForMockCompletionParams {
    /// Name of the mock location instance to wait for (optional, defaults to default instance)
    #[serde(default)]
    pub name: Option<String>,

    /// Timeout in ms. If not provided, waits indefinitely until mock completes.
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartProfilingParams {
    #[serde(default)]
    pub sampling_interval_ms: Option<u64>,
    #[serde(default)]
    pub package: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopProfilingParams {
    #[serde(default)]
    pub save_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssertPerformanceParams {
    pub metric: String,
    pub limit: String, // e.g. "200MB", "60fps"
}

fn default_tolerance_meters() -> f64 {
    50.0
}

// Parameter types

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LaunchAppParamsInput {
    Struct(LaunchAppParams),
    String(String),
}

impl LaunchAppParamsInput {
    pub fn into_inner(self) -> LaunchAppParams {
        match self {
            Self::Struct(s) => s,
            Self::String(s) => LaunchAppParams {
                app_id: Some(s),
                clear_state: false,
                clear_keychain: false,
                stop_app: None,
                permissions: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LaunchAppParams {
    #[serde(default)]
    pub clear_state: bool,

    /// Clear iOS Keychain data (simulator only)
    #[serde(default)]
    pub clear_keychain: bool,

    /// Stop app before launching (default: true)
    #[serde(default)]
    pub stop_app: Option<bool>,

    /// Permissions to set (e.g. { all: deny }, { notifications: allow })
    #[serde(default)]
    pub permissions: Option<HashMap<String, String>>,

    #[serde(default, alias = "url")]
    pub app_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TapParams {
    #[serde(default)]
    pub text: Option<String>,

    #[serde(default)]
    pub regex: Option<String>,

    #[serde(default)]
    pub relative: Option<RelativeParams>,

    #[serde(default)]
    pub id: Option<String>,

    #[serde(default)]
    pub css: Option<String>,

    #[serde(default)]
    pub xpath: Option<String>,
    #[serde(default)]
    pub role: Option<String>,

    #[serde(default)]
    pub placeholder: Option<String>,

    #[serde(default)]
    pub point: Option<String>, // "x,y" format

    #[serde(default)]
    pub index: Option<u32>,

    /// Element class/type (e.g., "EditText", "Button")
    #[serde(default, alias = "type")]
    pub element_type: Option<String>,

    #[serde(default)]
    pub image: Option<String>, // Path to image file for template matching

    /// Region to search for image: top-left, top-right, bottom-left, bottom-right, etc.
    #[serde(default, alias = "imageRegion")]
    pub image_region: Option<String>,

    #[serde(default)]
    pub optional: bool,

    /// Wait and retry tap if the view hierarchy doesn't change (default: true)
    #[serde(default)]
    pub retry_tap_if_no_change: Option<bool>,

    /// Require exact text match (case-sensitive), disable case-insensitive fallback
    #[serde(default)]
    pub exact: bool,

    // Relative position aliases (shorthand for relative param)
    #[serde(default, alias = "rightOf")]
    pub right_of: Option<String>,

    #[serde(default, alias = "leftOf")]
    pub left_of: Option<String>,

    #[serde(default)]
    pub above: Option<String>,

    #[serde(default)]
    pub below: Option<String>,
}

/// Tap element by type and index (e.g., tap 2nd EditText)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TapAtParams {
    /// Element class/type (e.g., "EditText", "Button", "input")
    #[serde(alias = "type")]
    pub element_type: String,

    /// 0-based index of the element
    #[serde(default)]
    pub index: u32,
}

/// Input text at element by type and index
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputAtParams {
    /// Element class/type (e.g., "EditText", "input")
    #[serde(alias = "type")]
    pub element_type: String,

    /// 0-based index of the element
    #[serde(default)]
    pub index: u32,

    /// Text to input
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EraseTextParams {
    #[serde(default)]
    pub char_count: Option<u32>,
}

/// Parameters for inputText command
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InputTextParams {
    /// Text to input
    pub text: String,
    /// Enable Unicode input mode (uses ADBKeyBoard, slower but reliable)
    #[serde(default)]
    pub unicode: bool,
}

/// Input for InputText command - supports both simple string and struct
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputTextParamsInput {
    String(String),
    Struct(InputTextParams),
}

impl InputTextParamsInput {
    pub fn into_inner(self) -> InputTextParams {
        match self {
            Self::String(text) => InputTextParams {
                text,
                unicode: false, // default: fast mode
            },
            Self::Struct(s) => s,
        }
    }

    pub fn text(&self) -> &str {
        match self {
            Self::String(s) => s,
            Self::Struct(p) => &p.text,
        }
    }

    pub fn unicode(&self) -> bool {
        match self {
            Self::String(_) => false,
            Self::Struct(p) => p.unicode,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollParams {
    #[serde(default)]
    pub direction: Option<String>,

    #[serde(default)]
    pub distance: Option<u32>,

    #[serde(default)]
    pub duration: Option<u32>,

    #[serde(default)]
    pub from: Option<TapParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollUntilVisibleParams {
    pub text: Option<String>,
    pub regex: Option<String>,
    pub relative: Option<RelativeParams>,

    pub id: Option<String>,

    #[serde(default)]
    pub css: Option<String>,

    #[serde(default)]
    pub xpath: Option<String>,
    #[serde(default)]
    pub role: Option<String>,

    #[serde(default)]
    pub placeholder: Option<String>,

    #[serde(default, alias = "type")]
    pub element_type: Option<String>,

    #[serde(default)]
    pub image: Option<String>,

    #[serde(default = "default_max_scrolls")]
    pub max_scrolls: u32,

    #[serde(default)]
    pub direction: Option<String>,

    #[serde(default)]
    pub from: Option<TapParams>,
}

fn default_max_scrolls() -> u32 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AssertParams {
    #[serde(default)]
    pub text: Option<String>,

    #[serde(default)]
    pub regex: Option<String>,

    #[serde(default)]
    pub relative: Option<RelativeParams>,

    #[serde(default)]
    pub id: Option<String>,

    #[serde(default)]
    pub css: Option<String>,

    #[serde(default)]
    pub xpath: Option<String>,
    #[serde(default)]
    pub role: Option<String>,

    #[serde(default)]
    pub placeholder: Option<String>,

    #[serde(default, alias = "type")]
    pub element_type: Option<String>,

    #[serde(default)]
    pub image: Option<String>,

    #[serde(default)]
    pub index: Option<u32>,

    #[serde(default)]
    pub timeout: Option<u64>,

    #[serde(default)]
    pub contains_child: Option<Box<AssertParams>>,

    #[serde(default)]
    pub right_of: Option<String>,
    #[serde(default)]
    pub left_of: Option<String>,
    #[serde(default)]
    pub above: Option<String>,
    #[serde(default)]
    pub below: Option<String>,

    #[serde(default)]
    pub soft: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitParams {
    #[serde(default = "default_wait_ms")]
    pub ms: u64,
}

fn default_wait_ms() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepeatParams {
    #[serde(default)]
    pub times: Option<u32>,
    #[serde(default, rename = "while")]
    pub while_condition: Option<serde_json::Value>,
    pub commands: Vec<TestCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryParams {
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    pub commands: Vec<TestCommand>,
}

fn default_max_retries() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunFlowParams {
    pub path: Option<String>,

    /// Variables to pass to the nested flow
    #[serde(default, alias = "env")]
    pub vars: Option<HashMap<String, String>>,

    #[serde(default)]
    pub commands: Option<Vec<TestCommand>>,

    #[serde(default)]
    pub when: Option<serde_json::Value>,

    #[serde(default)]
    pub label: Option<String>,

    #[serde(default)]
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunFlowParamsInput {
    String(String),
    Struct(RunFlowParams),
}

impl RunFlowParamsInput {
    pub fn into_inner(self) -> RunFlowParams {
        match self {
            Self::String(s) => RunFlowParams {
                path: Some(s),
                vars: None,
                commands: None,
                when: None,
                label: None,
                optional: None,
            },
            Self::Struct(s) => s,
        }
    }
}

/// Set a variable for use in subsequent commands
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetVarParams {
    /// Variable name
    pub name: String,

    /// Variable value (can use ${var} syntax for substitution)
    pub value: String,
}

/// Assert a variable has expected value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssertVarParams {
    /// Variable name
    pub name: String,

    /// Expected value
    pub expected: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScreenshotParamsInput {
    Struct(ScreenshotParams),
    String(String),
}

impl ScreenshotParamsInput {
    pub fn into_inner(self) -> ScreenshotParams {
        match self {
            Self::Struct(s) => s,
            Self::String(s) => ScreenshotParams { path: s },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotParams {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RecordingParamsInput {
    Struct(RecordingParams),
    String(String),
}

impl RecordingParamsInput {
    pub fn into_inner(self) -> RecordingParams {
        match self {
            Self::Struct(s) => s,
            Self::String(s) => RecordingParams { path: s },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordingParams {
    pub path: String,
}

// GIF Recording params

/// Capture GIF frame params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureGifFrameParams {
    /// Frame name/identifier
    pub name: String,

    /// Crop region: "left%,top%,width%,height%"
    #[serde(default)]
    pub crop: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CaptureGifFrameParamsInput {
    Struct(CaptureGifFrameParams),
    String(String),
}

impl CaptureGifFrameParamsInput {
    pub fn into_inner(self) -> CaptureGifFrameParams {
        match self {
            Self::Struct(s) => s,
            Self::String(name) => CaptureGifFrameParams { name, crop: None },
        }
    }
}

/// GIF frame input - supports both simple name and name with custom delay
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GifFrameInput {
    /// Just frame name (uses default delay)
    Name(String),
    /// Frame with custom delay
    WithDelay { name: String, delay: u32 },
}

/// Build GIF params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildGifParams {
    /// List of frames (string or object with delay)
    pub frames: Vec<GifFrameInput>,

    /// Output file path
    pub output: String,

    /// Default delay per frame (ms), default 500
    #[serde(default = "default_gif_delay")]
    pub delay: u32,

    /// Resize width (keeps aspect ratio)
    #[serde(default)]
    pub width: Option<u32>,

    /// Resize height (keeps aspect ratio)
    #[serde(default)]
    pub height: Option<u32>,

    /// Quality: "low", "medium", "high" (default: medium)
    #[serde(default = "default_gif_quality")]
    pub quality: String,

    /// Max colors (2-256), default 128
    #[serde(default = "default_gif_colors")]
    pub colors: u16,

    /// Loop infinite (default true)
    #[serde(default = "default_gif_loop")]
    pub loop_gif: bool,

    /// Specific loop count (overrides loop_gif)
    #[serde(default)]
    pub loop_count: Option<u16>,
}

fn default_gif_delay() -> u32 {
    500
}
fn default_gif_quality() -> String {
    "medium".to_string()
}
fn default_gif_colors() -> u16 {
    128
}
fn default_gif_loop() -> bool {
    true
}

/// Start auto-capture GIF mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartGifCaptureParams {
    /// Capture interval in milliseconds
    #[serde(default = "default_capture_interval")]
    pub interval: u64,

    /// Maximum frames to capture
    #[serde(default = "default_max_frames")]
    pub max_frames: u32,

    /// Resize width for captured frames
    #[serde(default)]
    pub width: Option<u32>,
}

fn default_capture_interval() -> u64 {
    200
}
fn default_max_frames() -> u32 {
    150
}

/// Stop auto-capture and build GIF
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopGifCaptureParams {
    /// Output GIF path
    pub output: String,

    /// Frame delay in ms (default: uses capture interval)
    #[serde(default)]
    pub delay: Option<u32>,

    /// Quality: "low", "medium", "high"
    #[serde(default = "default_gif_quality")]
    pub quality: String,

    /// Loop count (None = infinite)
    #[serde(default)]
    pub loop_count: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportParams {
    pub path: String,

    #[serde(default = "default_report_format")]
    pub format: String,
}

fn default_report_format() -> String {
    "json".to_string()
}

// Web-specific params (Future)

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateParams {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkParams {
    #[serde(default)]
    pub wifi: Option<bool>,
    #[serde(default)]
    pub data: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundAppParams {
    pub app_id: Option<String>,
    #[serde(default = "default_background_duration")]
    pub duration_ms: u64,
}

fn default_background_duration() -> u64 {
    5000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrientationParams {
    pub mode: Orientation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClickParams {
    #[serde(default)]
    pub selector: Option<String>,

    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeParams {
    pub text: String,

    #[serde(default)]
    pub selector: Option<String>,
}

impl TestCommand {
    /// Get a display name for the command
    pub fn display_name(&self) -> String {
        match self {
            TestCommand::LaunchApp(p_input) => {
                let p = p_input.clone().map(|pi| pi.into_inner());
                if let Some(params) = p {
                    let mut parts = Vec::new();
                    if let Some(app_id) = &params.app_id {
                        parts.push(format!("\"{}\"", app_id));
                    }
                    if params.clear_state {
                        parts.push("clearState".to_string());
                    }
                    if params.clear_keychain {
                        parts.push("clearKeychain".to_string());
                    }
                    if parts.is_empty() {
                        "launchApp".to_string()
                    } else {
                        format!("launchApp({})", parts.join(", "))
                    }
                } else {
                    "launchApp".to_string()
                }
            }
            TestCommand::StopApp => "stopApp".to_string(),
            TestCommand::TapOn(p_input) => {
                let p = p_input.clone().into_inner();
                if let Some(text) = &p.text {
                    if let Some(idx) = p.index {
                        format!("tapOn(text: \"{}\", index: {})", text, idx)
                    } else {
                        format!("tapOn(text: \"{}\")", text)
                    }
                } else if let Some(point) = &p.point {
                    format!("tapOn(point: {})", point)
                } else if let Some(id) = &p.id {
                    format!("tapOn(id: \"{}\")", id)
                } else if let Some(el_type) = &p.element_type {
                    let idx = p.index.unwrap_or(0);
                    format!("tapOn(type: \"{}\", index: {})", el_type, idx)
                } else if let Some(regex) = &p.regex {
                    format!("tapOn(regex: \"{}\")", regex)
                } else if let Some(css) = &p.css {
                    format!("tapOn(css: \"{}\")", css)
                } else if let Some(xpath) = &p.xpath {
                    format!("tapOn(xpath: \"{}\")", xpath)
                } else if let Some(placeholder) = &p.placeholder {
                    format!("tapOn(placeholder: \"{}\")", placeholder)
                } else if let Some(role) = &p.role {
                    format!("tapOn(role: \"{}\")", role)
                } else if let Some(image) = &p.image {
                    format!("tapOn(image: \"{}\")", image)
                } else if let Some(rel) = &p.relative {
                    // Show direction and anchor text for better clarity
                    let (dir, anchor) = if let Some(ref text) = rel.right_of {
                        ("rightOf", text.as_str())
                    } else if let Some(ref text) = rel.left_of {
                        ("leftOf", text.as_str())
                    } else if let Some(ref text) = rel.above {
                        ("above", text.as_str())
                    } else if let Some(ref text) = rel.below {
                        ("below", text.as_str())
                    } else {
                        ("relative", "")
                    };
                    if anchor.is_empty() {
                        format!("tapOn(relative: {})", dir)
                    } else {
                        format!("tapOn({} \"{}\")", dir, anchor)
                    }
                } else {
                    "tapOn".to_string()
                }
            }
            TestCommand::LongPressOn(p_input) => {
                let p = p_input.clone().into_inner();
                if let Some(text) = &p.text {
                    format!("longPressOn(text: \"{}\")", text)
                } else if let Some(id) = &p.id {
                    format!("longPressOn(id: \"{}\")", id)
                } else if let Some(el_type) = &p.element_type {
                    let idx = p.index.unwrap_or(0);
                    format!("longPressOn(type: \"{}\", index: {})", el_type, idx)
                } else if let Some(point) = &p.point {
                    format!("longPressOn(point: {})", point)
                } else {
                    "longPressOn".to_string()
                }
            }
            TestCommand::DoubleTapOn(p_input) => {
                let p = p_input.clone().into_inner();
                if let Some(text) = &p.text {
                    format!("doubleTapOn(text: \"{}\")", text)
                } else if let Some(id) = &p.id {
                    format!("doubleTapOn(id: \"{}\")", id)
                } else if let Some(el_type) = &p.element_type {
                    let idx = p.index.unwrap_or(0);
                    format!("doubleTapOn(type: \"{}\", index: {})", el_type, idx)
                } else if let Some(point) = &p.point {
                    format!("doubleTapOn(point: {})", point)
                } else {
                    "doubleTapOn".to_string()
                }
            }
            TestCommand::InputText(params_input) => {
                format!("inputText(\"{}\")", params_input.text())
            }
            TestCommand::EraseText(_) => "eraseText".to_string(),
            TestCommand::HideKeyboard => "hideKeyboard".to_string(),
            TestCommand::SwipeLeft => "swipeLeft".to_string(),
            TestCommand::SwipeRight => "swipeRight".to_string(),
            TestCommand::SwipeUp => "swipeUp".to_string(),
            TestCommand::SwipeDown => "swipeDown".to_string(),
            TestCommand::ManualScroll(_) => "scroll".to_string(),
            TestCommand::ScrollUntilVisible(p_input) => {
                let p = p_input.clone().into_inner();
                if let Some(text) = &p.text {
                    format!("scrollUntilVisible(text: \"{}\")", text)
                } else if let Some(id) = &p.id {
                    format!("scrollUntilVisible(id: \"{}\")", id)
                } else if let Some(regex) = &p.regex {
                    format!("scrollUntilVisible(regex: \"{}\")", regex)
                } else if let Some(el_type) = &p.element_type {
                    format!("scrollUntilVisible(type: \"{}\")", el_type)
                } else if let Some(image) = &p.image {
                    format!("scrollUntilVisible(image: \"{}\")", image)
                } else {
                    "scrollUntilVisible".to_string()
                }
            }
            TestCommand::AssertVisible(p_input) => {
                let p = p_input.clone().into_inner();
                if let Some(text) = &p.text {
                    format!("assertVisible(text: \"{}\")", text)
                } else if let Some(id) = &p.id {
                    format!("assertVisible(id: \"{}\")", id)
                } else if let Some(regex) = &p.regex {
                    format!("assertVisible(regex: \"{}\")", regex)
                } else if let Some(el_type) = &p.element_type {
                    format!("assertVisible(type: \"{}\")", el_type)
                } else if let Some(css) = &p.css {
                    format!("assertVisible(css: \"{}\")", css)
                } else if let Some(image) = &p.image {
                    format!("assertVisible(image: \"{}\")", image)
                } else {
                    "assertVisible".to_string()
                }
            }
            TestCommand::WaitUntilVisible(p_input) => {
                let p = p_input.clone().into_inner();
                if let Some(text) = &p.text {
                    format!("waitUntilVisible(text: \"{}\")", text)
                } else if let Some(id) = &p.id {
                    format!("waitUntilVisible(id: \"{}\")", id)
                } else if let Some(regex) = &p.regex {
                    format!("waitUntilVisible(regex: \"{}\")", regex)
                } else if let Some(el_type) = &p.element_type {
                    format!("waitUntilVisible(type: \"{}\")", el_type)
                } else if let Some(css) = &p.css {
                    format!("waitUntilVisible(css: \"{}\")", css)
                } else if let Some(image) = &p.image {
                    format!("waitUntilVisible(image: \"{}\")", image)
                } else {
                    "waitUntilVisible".to_string()
                }
            }
            TestCommand::AssertNotVisible(p_input) => {
                let p = p_input.clone().into_inner();
                if let Some(text) = &p.text {
                    format!("assertNotVisible(text: \"{}\")", text)
                } else if let Some(id) = &p.id {
                    format!("assertNotVisible(id: \"{}\")", id)
                } else if let Some(regex) = &p.regex {
                    format!("assertNotVisible(regex: \"{}\")", regex)
                } else {
                    "assertNotVisible".to_string()
                }
            }
            TestCommand::WaitUntilNotVisible(p_input) => {
                let p = p_input.clone().into_inner();
                if let Some(text) = &p.text {
                    format!("waitNotSee(text: \"{}\")", text)
                } else if let Some(id) = &p.id {
                    format!("waitNotSee(id: \"{}\")", id)
                } else if let Some(regex) = &p.regex {
                    format!("waitNotSee(regex: \"{}\")", regex)
                } else {
                    "waitNotSee".to_string()
                }
            }
            TestCommand::WaitForAnimationToEnd => "waitForAnimationToEnd".to_string(),
            TestCommand::Wait(p_input) => {
                let p = p_input.clone().into_inner();
                format!("wait({}ms)", p.ms)
            }
            TestCommand::Repeat(p) => {
                if let Some(times) = p.times {
                    format!("repeat({} times)", times)
                } else if p.while_condition.is_some() {
                    "repeat(while)".to_string()
                } else {
                    "repeat".to_string()
                }
            }
            TestCommand::Retry(p) => format!("retry(max: {})", p.max_retries),
            TestCommand::RunFlow(p_input) => {
                let p = p_input.clone().into_inner();
                if let Some(path) = &p.path {
                    format!("runFlow(\"{}\")", path)
                } else if p.commands.is_some() {
                    "runFlow(inline)".to_string()
                } else {
                    "runFlow".to_string()
                }
            }
            TestCommand::TakeScreenshot(_) => "screenshot".to_string(),
            TestCommand::StartRecording(_) => "startRecording".to_string(),
            TestCommand::StopRecording => "stopRecording".to_string(),
            TestCommand::ExportReport(_) => "exportReport".to_string(),
            TestCommand::Back => "back".to_string(),
            TestCommand::PressHome => "pressHome".to_string(),
            TestCommand::Navigate(_) => "navigate".to_string(),
            TestCommand::Click(_) => "click".to_string(),
            TestCommand::Type(p) => {
                let sel = p.selector.as_deref().unwrap_or("focused");
                format!("type(\"{}\", \"{}\")", p.text, sel)
            }
            TestCommand::TapAt(p) => {
                format!("tapAt({}[{}])", p.element_type, p.index)
            }
            TestCommand::InputAt(p) => {
                format!("inputAt({}[{}], \"{}\")", p.element_type, p.index, p.text)
            }
            TestCommand::RightClick(p) => {
                if let Some(text) = &p.text {
                    format!("rightClick(text: \"{}\")", text)
                } else if let Some(id) = &p.id {
                    format!("rightClick(id: \"{}\")", id)
                } else if let Some(el_type) = &p.element_type {
                    format!("rightClick(type: \"{}\")", el_type)
                } else if let Some(css) = &p.css {
                    format!("rightClick(css: \"{}\")", css)
                } else {
                    "rightClick".to_string()
                }
            }
            TestCommand::SetVar(p) => {
                format!("setVar({} = \"{}\")", p.name, p.value)
            }
            TestCommand::AssertVar(p) => {
                format!("assertVar({} == \"{}\")", p.name, p.expected)
            }
            TestCommand::Generate(p) => {
                format!("generate({}: {})", p.name, p.data_type)
            }
            TestCommand::HttpRequest(p) => {
                format!("httpRequest({} {})", p.method, p.url)
            }
            TestCommand::OpenLink(url) => {
                format!("openLink(\"{}\")", url)
            }
            TestCommand::AssertScreenshot(name) => {
                format!("assertScreenshot(\"{}\")", name)
            }
            TestCommand::RunScript(p_input) => {
                let p = p_input.clone().into_inner();
                format!("runScript(\"{}\")", p.command)
            }
            TestCommand::Conditional(p) => {
                if let Some(visible) = &p.condition.visible {
                    format!("conditional(visible: \"{}\")", visible)
                } else if let Some(visible_regex) = &p.condition.visible_regex {
                    format!("conditional(visibleRegex: \"{}\")", visible_regex)
                } else if let Some(not_visible) = &p.condition.not_visible {
                    format!("conditional(notVisible: \"{}\")", not_visible)
                } else if let Some(not_visible_regex) = &p.condition.not_visible_regex {
                    format!("conditional(notVisibleRegex: \"{}\")", not_visible_regex)
                } else {
                    "conditional".to_string()
                }
            }
            TestCommand::MockLocation(p_input) => {
                let p = p_input.clone().into_inner();
                format!("mockLocation(\"{}\")", p.file)
            }
            TestCommand::StopMockLocation => "stopMockLocation".to_string(),
            TestCommand::MockLocationControl(p) => {
                if let Some(speed) = p.speed {
                    format!("mockLocationControl(speed: {})", speed)
                } else if p.pause == Some(true) {
                    "mockLocationControl(pause)".to_string()
                } else if p.resume == Some(true) {
                    "mockLocationControl(resume)".to_string()
                } else {
                    "mockLocationControl".to_string()
                }
            }
            TestCommand::AssertColor(p) => {
                format!("assertColor({}, \"{}\")", p.point, p.color)
            }
            TestCommand::CaptureGifFrame(p_input) => {
                let p = p_input.clone().into_inner();
                format!("captureGifFrame(\"{}\")", p.name)
            }
            TestCommand::BuildGif(p) => {
                format!("buildGif({} frames -> \"{}\")", p.frames.len(), p.output)
            }
            TestCommand::StartGifCapture(p) => {
                format!(
                    "startGifCapture(interval: {}ms, max: {})",
                    p.interval, p.max_frames
                )
            }
            TestCommand::StopGifCapture(p) => {
                format!("stopGifCapture(\"{}\")", p.output)
            }
            TestCommand::RotateScreen(p_input) => {
                let p = p_input.clone().into_inner();
                format!("rotate(\"{}\")", p.mode)
            }
            TestCommand::PressKey(k) => format!("press(\"{}\")", k),
            TestCommand::PushFile(p) => format!("pushFile({} -> {})", p.source, p.destination),
            TestCommand::PullFile(p) => format!("pullFile({} -> {})", p.source, p.destination),
            TestCommand::ClearAppData(pkg) => format!("clearAppData({})", pkg),
            TestCommand::SetClipboard(t) => format!("setClipboard(\"{}\")", t),
            TestCommand::GetClipboard(p) => format!("getClipboard({})", p.name),
            TestCommand::AssertClipboard(e) => format!("assertClipboard(\"{}\")", e),

            TestCommand::AssertTrue(p) => match p {
                AssertTrueParams::Condition(c) => format!("assertTrue({})", c.condition),
                AssertTrueParams::Expression(expr) => format!("assertTrue({})", expr),
            },
            TestCommand::EvalScript(expr) => format!("evalScript({})", expr),
            TestCommand::CopyTextFrom(p) => {
                if let Some(text) = &p.text {
                    format!("copyTextFrom(text: \"{}\")", text)
                } else if let Some(id) = &p.id {
                    format!("copyTextFrom(id: \"{}\")", id)
                } else {
                    "copyTextFrom".to_string()
                }
            }
            TestCommand::PasteText => "pasteText".to_string(),
            TestCommand::InputRandomEmail => "inputRandomEmail".to_string(),
            TestCommand::InputRandomNumber(p) => {
                if let Some(params) = p {
                    if let Some(len) = params.length {
                        format!("inputRandomNumber(length: {})", len)
                    } else {
                        "inputRandomNumber".to_string()
                    }
                } else {
                    "inputRandomNumber".to_string()
                }
            }
            TestCommand::InputRandomPersonName => "inputRandomPersonName".to_string(),
            TestCommand::InputRandomText(p) => {
                if let Some(params) = p {
                    if let Some(len) = params.length {
                        format!("inputRandomText(length: {})", len)
                    } else {
                        "inputRandomText".to_string()
                    }
                } else {
                    "inputRandomText".to_string()
                }
            }
            TestCommand::ExtendedWaitUntil(p) => {
                format!("extendedWaitUntil(timeout: {}ms)", p.timeout)
            }
            TestCommand::DbQuery(p) => {
                format!("dbQuery(query: \"{}\")", p.query)
            }
            TestCommand::SetNetwork(p) => {
                let mut parts = Vec::new();
                if let Some(w) = p.wifi {
                    parts.push(format!("wifi: {}", w));
                }
                if let Some(d) = p.data {
                    parts.push(format!("data: {}", d));
                }
                format!("setNetwork({})", parts.join(", "))
            }
            TestCommand::ToggleAirplaneMode => "airplaneMode".to_string(),
            TestCommand::OpenNotifications => "openNotifications".to_string(),
            TestCommand::OpenQuickSettings => "openQuickSettings".to_string(),
            TestCommand::SetVolume(v) => format!("setVolume({})", v),
            TestCommand::LockDevice => "lockDevice".to_string(),
            TestCommand::UnlockDevice => "unlockDevice".to_string(),
            TestCommand::InstallApp(path) => format!("installApp(\"{}\")", path),
            TestCommand::UninstallApp(pkg) => format!("uninstallApp(\"{}\")", pkg),
            TestCommand::BackgroundApp(p) => format!(
                "backgroundApp({}, {}ms)",
                p.app_id.as_deref().unwrap_or("current"),
                p.duration_ms
            ),
            TestCommand::SetOrientation(p) => format!("setOrientation({:?})", p.mode),
            TestCommand::WaitForLocation(p) => {
                format!(
                    "waitForLocation({:.4}, {:.4}, tol: {:.1})",
                    p.lat, p.lon, p.tolerance
                )
            }
            TestCommand::WaitForMockCompletion(p) => {
                if let Some(t) = p.timeout {
                    format!("waitForMockCompletion(timeout: {}ms)", t)
                } else {
                    "waitForMockCompletion".to_string()
                }
            }

            // Performance & Load Testing
            TestCommand::StartProfiling(_) => "startProfiling".to_string(),
            TestCommand::StopProfiling(_) => "stopProfiling".to_string(),
            TestCommand::AssertPerformance(p) => {
                format!("assertPerformance({} check {})", p.metric, p.limit)
            }
            TestCommand::SetCpuThrottling(rate) => format!("setCpuThrottling({}x)", rate),
            TestCommand::SetNetworkConditions(profile) => {
                format!("setNetworkConditions(\"{}\")", profile)
            }
            TestCommand::SelectDisplay(id) => format!("selectDisplay({})", id),
            TestCommand::SetLocale(locale) => format!("setLocale(\"{}\")", locale),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateParams {
    pub name: String,

    #[serde(rename = "type")]
    pub data_type: String, // uuid, email, phone, name, address, number, date

    #[serde(default)]
    pub format: Option<String>, // format string for date, or min-max for number "1-100"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestParams {
    pub url: String,
    pub method: String, // GET, POST, PUT, DELETE

    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,

    #[serde(default)]
    pub body: Option<serde_yaml::Value>, // JSON/YAML value or string

    #[serde(default)]
    pub save_response: Option<HashMap<String, String>>, // map json path -> var name

    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunScriptParamsInput {
    Struct(RunScriptParams),
    String(String),
}

impl RunScriptParamsInput {
    pub fn into_inner(self) -> RunScriptParams {
        match self {
            Self::Struct(s) => s,
            Self::String(s) => RunScriptParams {
                command: s,
                args: None,
                save_output: None,
                timeout_ms: None,
                fail_on_error: false,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScriptParams {
    pub command: String,

    #[serde(default)]
    pub args: Option<Vec<String>>,

    #[serde(default)]
    pub save_output: Option<String>, // variable name to save stdout

    #[serde(default)]
    pub timeout_ms: Option<u64>,

    #[serde(default)]
    pub fail_on_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalParams {
    pub condition: Condition,
    pub then: serde_yaml::Value,
    #[serde(default, rename = "else")]
    pub else_cmd: Option<serde_yaml::Value>,
}

/// Speed simulation mode
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SpeedMode {
    /// Constant speed (linear interpolation)
    #[default]
    Linear,
    /// Speed with random noise within +/- speedNoise km/h
    Noise,
}

/// Mock location parameters for GPS simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockLocationParams {
    /// Unique name for this mock instance (optional, allows running multiple mocks)
    #[serde(default)]
    pub name: Option<String>,

    /// Path to GPX, KML, or JSON file
    pub file: String,

    /// Override speed in km/h (ignores timestamps in file)
    #[serde(default)]
    pub speed: Option<f64>,

    /// Speed simulation mode: linear (constant) or noise (variable)
    #[serde(default)]
    pub speed_mode: SpeedMode,

    /// Speed noise range in km/h (used when speedMode is noise), e.g., 5.0 means +/- 5 km/h
    #[serde(default)]
    pub speed_noise: Option<f64>,

    /// Loop the route continuously
    #[serde(default, rename = "loop")]
    pub loop_route: bool,

    /// Start from specific waypoint index (0-based)
    #[serde(default)]
    pub start_index: Option<u32>,

    /// Update interval in milliseconds (default: 1000)
    #[serde(default)]
    pub interval_ms: Option<u64>,
}

/// Mock location control parameters for dynamic speed adjustment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockLocationControlParams {
    /// Name of the mock instance to control (optional, defaults to default instance)
    #[serde(default)]
    pub name: Option<String>,

    /// New speed in km/h
    #[serde(default)]
    pub speed: Option<f64>,

    /// Speed simulation mode
    #[serde(default)]
    pub speed_mode: Option<SpeedMode>,

    /// Speed noise range in km/h
    #[serde(default)]
    pub speed_noise: Option<f64>,

    /// Pause the mock location playback
    #[serde(default)]
    pub pause: Option<bool>,

    /// Resume the mock location playback
    #[serde(default)]
    pub resume: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MockLocationParamsInput {
    Struct(MockLocationParams),
    String(String),
}

impl MockLocationParamsInput {
    pub fn into_inner(self) -> MockLocationParams {
        match self {
            Self::Struct(s) => s,
            Self::String(file) => MockLocationParams {
                name: None,
                file,
                speed: None,
                speed_mode: SpeedMode::Linear,
                speed_noise: None,
                loop_route: false,
                start_index: None,
                interval_ms: None,
            },
        }
    }
}

/// Assert color at a specific point on screen
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssertColorParams {
    /// Point to check: "540,960" (absolute) or "50%,50%" (percentage)
    pub point: String,

    /// Expected color: "#4CAF50" (hex) or "green" (named)
    pub color: String,

    /// Color tolerance in percentage (0-100), default 10%
    #[serde(default = "default_color_tolerance")]
    pub tolerance: f64,
}

fn default_color_tolerance() -> f64 {
    10.0
}

impl AssertColorParams {
    /// Parse point string to (x, y) coordinates
    /// Supports: "540,960" (absolute) or "50%,50%" (percentage of screen)
    pub fn parse_point(&self, screen_width: u32, screen_height: u32) -> Option<(i32, i32)> {
        let parts: Vec<&str> = self.point.split(',').collect();
        if parts.len() != 2 {
            return None;
        }

        let x_str = parts[0].trim();
        let y_str = parts[1].trim();

        let x = if x_str.ends_with('%') {
            let pct: f64 = x_str.trim_end_matches('%').parse().ok()?;
            (screen_width as f64 * pct / 100.0) as i32
        } else {
            x_str.parse().ok()?
        };

        let y = if y_str.ends_with('%') {
            let pct: f64 = y_str.trim_end_matches('%').parse().ok()?;
            (screen_height as f64 * pct / 100.0) as i32
        } else {
            y_str.parse().ok()?
        };

        Some((x, y))
    }

    /// Parse color string to RGB values
    /// Supports hex "#RRGGBB" or named colors
    pub fn parse_color(&self) -> Option<(u8, u8, u8)> {
        let color = self.color.trim().to_lowercase();

        // Named colors
        match color.as_str() {
            "red" => return Some((255, 0, 0)),
            "green" => return Some((0, 255, 0)),
            "blue" => return Some((0, 0, 255)),
            "white" => return Some((255, 255, 255)),
            "black" => return Some((0, 0, 0)),
            "yellow" => return Some((255, 255, 0)),
            "orange" => return Some((255, 165, 0)),
            "gray" | "grey" => return Some((128, 128, 128)),
            "cyan" => return Some((0, 255, 255)),
            "magenta" => return Some((255, 0, 255)),
            _ => {}
        }

        // Hex color #RRGGBB or #RGB
        if color.starts_with('#') {
            let hex = color.trim_start_matches('#');
            if hex.len() == 6 {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                return Some((r, g, b));
            } else if hex.len() == 3 {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
                return Some((r, g, b));
            }
        }

        None
    }

    /// Calculate color distance as percentage (0-100)
    pub fn color_distance(c1: (u8, u8, u8), c2: (u8, u8, u8)) -> f64 {
        let dr = (c1.0 as f64 - c2.0 as f64).powi(2);
        let dg = (c1.1 as f64 - c2.1 as f64).powi(2);
        let db = (c1.2 as f64 - c2.2 as f64).powi(2);
        // Max possible distance is sqrt(3 * 255^2) = 441.67
        // Normalize to 0-100%
        ((dr + dg + db).sqrt() / 441.67) * 100.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    #[serde(default)]
    pub visible: Option<String>,
    #[serde(default)]
    pub visible_regex: Option<String>,
    #[serde(default)]
    pub not_visible: Option<String>,
    #[serde(default)]
    pub not_visible_regex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RelativeParams {
    pub right_of: Option<String>,
    pub left_of: Option<String>,
    pub above: Option<String>,
    pub below: Option<String>,
    #[serde(alias = "maxDistance")]
    pub max_dist: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum RelativeDirection {
    LeftOf,
    RightOf,
    Above,
    #[default]
    Below,
    Near,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AnchorParams {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub css: Option<String>,
    #[serde(default)]
    pub xpath: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub index: Option<u32>,
}

impl Default for WaitParams {
    fn default() -> Self {
        Self {
            ms: default_wait_ms(),
        }
    }
}

impl Default for ScrollUntilVisibleParams {
    fn default() -> Self {
        Self {
            text: None,
            regex: None,
            relative: None,
            id: None,
            css: None,
            xpath: None,
            placeholder: None,
            role: None, // Updated this line
            max_scrolls: default_max_scrolls(),
            direction: None,
            element_type: None,
            image: None,
            from: None,
        }
    }
}

fn is_regex_string(s: &str) -> bool {
    s.contains(".*")
        || s.contains(".+")
        || (s.starts_with('^') && s.ends_with('$'))
        || s.contains(r"\d+")
        || s.contains(r"\d{")
        || s.contains('[')
        || s.contains('(')
        || s.contains('|')
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TapParamsInput {
    String(String),
    Struct(TapParams),
}
impl TapParamsInput {
    pub fn into_inner(self) -> TapParams {
        match self {
            TapParamsInput::String(s) => {
                if is_regex_string(&s) {
                    TapParams {
                        regex: Some(s),
                        ..Default::default()
                    }
                } else {
                    TapParams {
                        text: Some(s),
                        ..Default::default()
                    }
                }
            }
            TapParamsInput::Struct(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssertParamsInput {
    String(String),
    Struct(AssertParams),
}
impl AssertParamsInput {
    pub fn into_inner(self) -> AssertParams {
        match self {
            AssertParamsInput::String(s) => {
                if is_regex_string(&s) {
                    AssertParams {
                        regex: Some(s),
                        ..Default::default()
                    }
                } else {
                    AssertParams {
                        text: Some(s),
                        ..Default::default()
                    }
                }
            }
            AssertParamsInput::Struct(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WaitParamsInput {
    Number(u64),
    Struct(WaitParams),
}
impl WaitParamsInput {
    pub fn into_inner(self) -> WaitParams {
        match self {
            WaitParamsInput::Number(n) => WaitParams { ms: n },
            WaitParamsInput::Struct(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScrollUntilVisibleInput {
    String(String),
    Struct(ScrollUntilVisibleParams),
}
impl ScrollUntilVisibleInput {
    pub fn into_inner(self) -> ScrollUntilVisibleParams {
        match self {
            ScrollUntilVisibleInput::String(s) => {
                if is_regex_string(&s) {
                    ScrollUntilVisibleParams {
                        regex: Some(s),
                        ..Default::default()
                    }
                } else {
                    ScrollUntilVisibleParams {
                        text: Some(s),
                        ..Default::default()
                    }
                }
            }
            ScrollUntilVisibleInput::Struct(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnchorParamsInput {
    String(String),
    Struct(AnchorParams),
}
impl AnchorParamsInput {
    pub fn into_inner(self) -> AnchorParams {
        match self {
            AnchorParamsInput::String(s) => {
                if is_regex_string(&s) {
                    AnchorParams {
                        regex: Some(s),
                        ..Default::default()
                    }
                } else {
                    AnchorParams {
                        text: Some(s),
                        ..Default::default()
                    }
                }
            }
            AnchorParamsInput::Struct(s) => s,
        }
    }
}
impl Default for AnchorParamsInput {
    fn default() -> Self {
        AnchorParamsInput::Struct(AnchorParams::default())
    }
}
// Device Control

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RotationParamsInput {
    Struct(RotationParams),
    String(String),
}

impl RotationParamsInput {
    pub fn into_inner(self) -> RotationParams {
        match self {
            Self::Struct(s) => s,
            Self::String(s) => RotationParams { mode: s },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RotationParams {
    /// "portrait" or "landscape"
    pub mode: String,
}

// File Management

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTransferParams {
    pub source: String,
    pub destination: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbQueryParams {
    pub connection: String,
    pub query: String,
    #[serde(default)]
    pub params: Option<Vec<String>>,
    #[serde(default)]
    pub save: Option<HashMap<String, String>>,
}
