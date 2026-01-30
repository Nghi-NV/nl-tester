use super::types::{
    AssertColorParams, AssertParams, AssertParamsInput, AssertVarParams, BuildGifParams,
    CaptureGifFrameParamsInput, ConditionalParams, GenerateParams, HttpRequestParams,
    InputAtParams, LaunchAppParams, MockLocationParamsInput, Platform, RepeatParams, ReportParams,
    RetryParams, ScrollUntilVisibleInput, ScrollUntilVisibleParams, SetVarParams, TapAtParams,
    TapParams, TapParamsInput, TestCommand, TestFlow, WaitParams, WaitParamsInput,
};
use anyhow::{Context, Result};
use std::path::Path;

/// Parse a YAML test file into a TestFlow
pub fn parse_test_file(path: &Path) -> Result<TestFlow> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    parse_yaml_content(&content, path)
}

/// Parse YAML content into a TestFlow
pub fn parse_yaml_content(content: &str, _source_path: &Path) -> Result<TestFlow> {
    // 1. Check for custom "---" separator format
    if content.contains("---") {
        let parts: Vec<&str> = content.split("---").collect();
        let (header, commands_yaml) = if parts.len() >= 2 {
            (parts[0].trim(), parts[1..].join("---"))
        } else {
            ("", content.to_string()) // Should not happen if contains ---
        };

        // Parse header
        let mut flow = if !header.is_empty() {
            parse_header(header, _source_path)?
        } else {
            TestFlow {
                app_id: None,
                url: None,
                platform: Some(Platform::default()),
                env: None,
                data: None,
                default_timeout_ms: None,
                commands: Vec::new(),
                tags: Vec::new(),
                speed: None,
                browser: None,
                close_when_finish: None,
            }
        };
        // Parse commands
        flow.commands = parse_commands(&commands_yaml)?;
        return Ok(flow);
    }

    // 2. Try parsing entire content as a list of commands (legacy simple format)
    if let Ok(commands) = parse_commands(content) {
        return Ok(TestFlow {
            app_id: None,
            url: None,
            platform: Some(Platform::default()),
            env: None,
            data: None,
            default_timeout_ms: None,
            commands,
            tags: Vec::new(),
            speed: None,
            browser: None,
            close_when_finish: None,
        });
    }

    // 3. Try parsing entire content as TestFlow struct (Map with commands field)
    // We need to deserialize into a Value first to check structure/fields manually or use TestFlow struct directly
    // But TestFlow struct uses `TestCommand` type which has custom logic?
    // Actually TestFlow derives Deserialize via serde, and TestCommand also does.
    // BUT TestCommand deserializer might not handle the short syntax if serde default is used.
    // My TestCommand parses from Map {key:val} correctly via normal serde.
    // The issue is `parse_command_value` logic handles string shortcuts like "- stopApp" which serde default enum deserializer might not (it expects String variant name).

    // To support `TestFlow` struct with shortcuts, we'd need custom deserializer for TestFlow or TestCommand.
    // For now, let's manually parse the map.

    let value: serde_yaml::Value =
        serde_yaml::from_str(content).context("Failed to parse YAML content")?;

    if let serde_yaml::Value::Mapping(map) = value {
        let mut flow = TestFlow {
            app_id: None,
            url: None,
            platform: Some(Platform::default()),
            env: None,
            data: None,
            default_timeout_ms: None,
            commands: Vec::new(),
            tags: Vec::new(),
            speed: None,
            browser: None,
            close_when_finish: None,
        };

        if let Some(val) = map.get(&serde_yaml::Value::String("data".to_string())) {
            if let Some(s) = val.as_str() {
                flow.data = Some(s.to_string());
            }
        }

        if let Some(val) = map.get(&serde_yaml::Value::String("tags".to_string())) {
            if let Ok(tags) = serde_yaml::from_value(val.clone()) {
                flow.tags = tags;
            }
        }

        if let Some(val) = map.get(&serde_yaml::Value::String("appId".to_string())) {
            if let Some(s) = val.as_str() {
                flow.app_id = Some(s.to_string());
            }
        }
        // ... extract other fields ... env, etc.
        if let Some(val) = map.get(&serde_yaml::Value::String("env".to_string())) {
            flow.env = serde_yaml::from_value(val.clone()).ok();
        }

        if let Some(val) = map.get(&serde_yaml::Value::String("commands".to_string())) {
            // Parse commands using our custom parser helper
            if let serde_yaml::Value::Sequence(seq) = val {
                let mut cmds = Vec::new();
                for item in seq {
                    if let Some(cmd) = parse_command_value(item)? {
                        cmds.push(cmd);
                    }
                }
                flow.commands = cmds;
            }
        } else if let Some(val) = map.get(&serde_yaml::Value::String("steps".to_string())) {
            // Support 'steps' alias
            if let serde_yaml::Value::Sequence(seq) = val {
                let mut cmds = Vec::new();
                for item in seq {
                    if let Some(cmd) = parse_command_value(item)? {
                        cmds.push(cmd);
                    }
                }
                flow.commands = cmds;
            }
        }
        return Ok(flow);
    }

    anyhow::bail!("Invalid YAML format: expected sequence or TestFlow map");
}

/// Parse the header section of a YAML test file
fn parse_header(header: &str, base_path: &Path) -> Result<TestFlow> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Header {
        #[serde(default)]
        app_id: Option<String>,
        #[serde(default)]
        url: Option<String>,
        #[serde(default)]
        platform: Option<Platform>,
        #[serde(default, alias = "vars", alias = "var")]
        env: Option<serde_yaml::Value>,
        #[serde(default)]
        data: Option<String>,
        #[serde(default, alias = "defaultTimeout")]
        default_timeout: Option<u64>,
        #[serde(default)]
        tags: Vec<String>,
        #[serde(default)]
        speed: Option<String>,
        #[serde(default)]
        browser: Option<String>,
        #[serde(default)]
        close_when_finish: Option<bool>,
    }

    let parsed: Header = serde_yaml::from_str(header).context("Failed to parse YAML header")?;

    // Process env
    let mut env_map = std::collections::HashMap::new();

    if let Some(env_val) = parsed.env {
        match env_val {
            serde_yaml::Value::Mapping(map) => {
                // Check if it's the special syntax: env: { file: "..." }
                if let Some(file_val) = map.get(&serde_yaml::Value::String("file".to_string())) {
                    if let Some(file_path_str) = file_val.as_str() {
                        // Resolve path relative to base_path
                        let env_path = if let Some(parent) = base_path.parent() {
                            parent.join(file_path_str)
                        } else {
                            Path::new(file_path_str).to_path_buf()
                        };

                        // Read .env file
                        let content = std::fs::read_to_string(&env_path).with_context(|| {
                            format!("Failed to read env file: {}", env_path.display())
                        })?;

                        // Parse .env content (simple KEY=VAL)
                        for line in content.lines() {
                            let line = line.trim();
                            if line.is_empty() || line.starts_with('#') {
                                continue;
                            }
                            if let Some((key, val)) = line.split_once('=') {
                                env_map.insert(key.trim().to_string(), val.trim().to_string());
                            }
                        }
                    }
                } else {
                    // Normal map syntax
                    for (k, v) in map {
                        if let (Some(k_str), Some(v_str)) = (k.as_str(), v.as_str()) {
                            env_map.insert(k_str.to_string(), v_str.to_string());
                        } else if let (Some(k_str), Some(v_num)) = (k.as_str(), v.as_u64()) {
                            env_map.insert(k_str.to_string(), v_num.to_string());
                        } else if let (Some(k_str), Some(v_bool)) = (k.as_str(), v.as_bool()) {
                            env_map.insert(k_str.to_string(), v_bool.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let env = if env_map.is_empty() {
        None
    } else {
        Some(env_map)
    };

    Ok(TestFlow {
        app_id: parsed.app_id,
        url: parsed.url,
        platform: parsed.platform,
        env,
        data: parsed.data,
        default_timeout_ms: parsed.default_timeout,
        commands: Vec::new(),
        tags: parsed.tags,
        speed: parsed.speed,
        browser: parsed.browser,
        close_when_finish: parsed.close_when_finish,
    })
}

/// Parse the commands section of a YAML test file
fn parse_commands(yaml: &str) -> Result<Vec<TestCommand>> {
    let yaml = yaml.trim();
    if yaml.is_empty() {
        return Ok(Vec::new());
    }

    // Parse as a list of YAML values
    let values: Vec<serde_yaml::Value> =
        serde_yaml::from_str(yaml).context("Failed to parse YAML commands")?;

    let mut commands = Vec::new();

    for value in values {
        if let Some(cmd) = parse_command_value(&value)? {
            commands.push(cmd);
        }
    }

    Ok(commands)
}

/// Parse a list of commands from a YAML value
pub fn parse_commands_from_value(value: &serde_yaml::Value) -> Result<Vec<TestCommand>> {
    match value {
        serde_yaml::Value::Sequence(seq) => {
            let mut cmds = Vec::new();
            for item in seq {
                if let Some(cmd) = parse_command_value(item)? {
                    cmds.push(cmd);
                }
            }
            Ok(cmds)
        }
        _ => {
            if let Some(cmd) = parse_command_value(value)? {
                Ok(vec![cmd])
            } else {
                Ok(Vec::new())
            }
        }
    }
}

/// Parse a single command from a YAML value
pub fn parse_command_value(value: &serde_yaml::Value) -> Result<Option<TestCommand>> {
    match value {
        // Simple string command like "- stopApp" or "- hideKeyboard"
        serde_yaml::Value::String(s) => parse_simple_command(s),

        // Command with parameters like "- tapOn:\n    text: 'Login'"
        serde_yaml::Value::Mapping(map) => {
            if map.len() != 1 {
                anyhow::bail!("Invalid command format: expected single key mapping");
            }

            let (key, params) = map.iter().next().unwrap();
            let cmd_name = key
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Command name must be a string"))?;

            // If params is null or empty map, try simple command first
            if params.is_null() || (params.is_mapping() && params.as_mapping().unwrap().is_empty())
            {
                if let Some(cmd) = parse_simple_command(cmd_name)? {
                    return Ok(Some(cmd));
                }
            }

            parse_command_with_params(cmd_name, params)
        }

        _ => {
            anyhow::bail!("Invalid command format: {:?}", value);
        }
    }
}

/// Parse a simple command without parameters
fn parse_simple_command(name: &str) -> Result<Option<TestCommand>> {
    let cmd = match name {
        "launchApp" | "open" => TestCommand::LaunchApp(None),
        "stopApp" | "stop" => TestCommand::StopApp,
        "hideKeyboard" | "hideKbd" => TestCommand::HideKeyboard,
        "swipeLeft" => TestCommand::SwipeLeft,
        "swipeRight" => TestCommand::SwipeRight,
        "swipeUp" => TestCommand::SwipeUp,
        "swipeDown" => TestCommand::SwipeDown,
        "waitForAnimationToEnd" => TestCommand::WaitForAnimationToEnd,
        "stopRecording" | "stopRecord" => TestCommand::StopRecording,
        "back" => TestCommand::Back,
        "pressHome" | "home" => TestCommand::PressHome,
        "eraseText" | "clear" => TestCommand::EraseText(None),
        "stopMockLocation" | "stopGps" => TestCommand::StopMockLocation,
        "pasteText" => TestCommand::PasteText,
        "inputRandomEmail" => TestCommand::InputRandomEmail,
        "inputRandomNumber" | "inputRandomPhoneNumber" => TestCommand::InputRandomNumber(None),
        "inputRandomPersonName" => TestCommand::InputRandomPersonName,
        "inputRandomText" => TestCommand::InputRandomText(None),
        "airplaneMode" | "toggleAirplaneMode" => TestCommand::ToggleAirplaneMode,
        "openNotifications" => TestCommand::OpenNotifications,
        "openQuickSettings" => TestCommand::OpenQuickSettings,
        "lockDevice" => TestCommand::LockDevice,
        "unlockDevice" => TestCommand::UnlockDevice,
        "click" => TestCommand::Click(crate::parser::types::ClickParams {
            selector: None,
            text: None,
        }),
        _ => return Ok(None),
    };

    Ok(Some(cmd))
}

/// Parse a command with parameters
fn parse_command_with_params(
    name: &str,
    params: &serde_yaml::Value,
) -> Result<Option<TestCommand>> {
    let cmd = match name {
        "launchApp" | "open" => {
            if params.is_string() {
                let s = params.as_str().unwrap().to_string();
                TestCommand::LaunchApp(Some(crate::parser::types::LaunchAppParamsInput::String(s)))
            } else {
                let p: LaunchAppParams =
                    serde_yaml::from_value(params.clone()).unwrap_or(LaunchAppParams {
                        clear_state: false,
                        clear_keychain: false,
                        stop_app: None,
                        permissions: None,
                        app_id: None,
                    });
                TestCommand::LaunchApp(Some(crate::parser::types::LaunchAppParamsInput::Struct(p)))
            }
        }

        "tapOn" | "tap" => {
            let p: TapParamsInput = if params.is_string() {
                serde_yaml::from_value(params.clone())?
            } else {
                let inner: TapParams = serde_yaml::from_value(params.clone())?;
                TapParamsInput::Struct(inner)
            };
            TestCommand::TapOn(p)
        }

        "longPressOn" | "longPress" => {
            let p: TapParamsInput = if params.is_string() {
                serde_yaml::from_value(params.clone())?
            } else {
                let inner: TapParams = serde_yaml::from_value(params.clone())?;
                TapParamsInput::Struct(inner)
            };
            TestCommand::LongPressOn(p)
        }

        "doubleTapOn" | "doubleTap" => {
            let p: TapParamsInput = if params.is_string() {
                serde_yaml::from_value(params.clone())?
            } else {
                let inner: TapParams = serde_yaml::from_value(params.clone())?;
                TapParamsInput::Struct(inner)
            };
            TestCommand::DoubleTapOn(p)
        }

        "inputText" | "write" | "type" => {
            // "type" can be InputText(String) or Type(TypeParams)
            // Try as simple string first (InputText)
            if let Ok(text) = serde_yaml::from_value::<String>(params.clone()) {
                TestCommand::InputText(crate::parser::types::InputTextParamsInput::String(text))
            } else if let Ok(p) =
                serde_yaml::from_value::<crate::parser::types::InputTextParams>(params.clone())
            {
                // If it parses as InputTextParams (struct with text/unicode), use it
                TestCommand::InputText(crate::parser::types::InputTextParamsInput::Struct(p))
            } else if let Ok(p) =
                serde_yaml::from_value::<crate::parser::types::TypeParams>(params.clone())
            {
                // If it parses as TypeParams (struct), use Type command
                TestCommand::Type(p)
            } else {
                // Fallback to error from string parsing
                let text: String = serde_yaml::from_value(params.clone())?;
                TestCommand::InputText(crate::parser::types::InputTextParamsInput::String(text))
            }
        }

        "eraseText" => {
            let p = serde_yaml::from_value(params.clone()).ok();
            TestCommand::EraseText(p)
        }

        "swipe" => {
            let p = serde_yaml::from_value(params.clone()).ok();
            TestCommand::ManualScroll(p)
        }

        "scrollUntilVisible" | "scrollTo" => {
            let p: ScrollUntilVisibleInput = if params.is_string() {
                serde_yaml::from_value(params.clone())?
            } else {
                let inner: ScrollUntilVisibleParams = serde_yaml::from_value(params.clone())?;
                ScrollUntilVisibleInput::Struct(inner)
            };
            TestCommand::ScrollUntilVisible(p)
        }

        "assertVisible" | "see" => {
            let p: AssertParamsInput = if params.is_string() {
                serde_yaml::from_value(params.clone())?
            } else {
                let inner: AssertParams = serde_yaml::from_value(params.clone())?;
                AssertParamsInput::Struct(inner)
            };
            TestCommand::AssertVisible(p)
        }

        "assertNotVisible" | "notSee" => {
            let p: AssertParamsInput = if params.is_string() {
                serde_yaml::from_value(params.clone())?
            } else {
                let inner: AssertParams = serde_yaml::from_value(params.clone())?;
                AssertParamsInput::Struct(inner)
            };
            TestCommand::AssertNotVisible(p)
        }

        "waitUntilVisible" | "waitSee" => {
            let p: AssertParamsInput = if params.is_string() {
                serde_yaml::from_value(params.clone())?
            } else {
                let inner: AssertParams = serde_yaml::from_value(params.clone())?;
                AssertParamsInput::Struct(inner)
            };
            TestCommand::WaitUntilVisible(p)
        }

        "waitUntilNotVisible" | "waitNotSee" => {
            let p: AssertParamsInput = if params.is_string() {
                serde_yaml::from_value(params.clone())?
            } else {
                let inner: AssertParams = serde_yaml::from_value(params.clone())?;
                AssertParamsInput::Struct(inner)
            };
            TestCommand::WaitUntilNotVisible(p)
        }

        "wait" | "await" => {
            let p_input = if let Some(ms) = params.as_u64() {
                WaitParamsInput::Number(ms)
            } else {
                let inner: WaitParams = serde_yaml::from_value(params.clone())?;
                WaitParamsInput::Struct(inner)
            };
            TestCommand::Wait(p_input)
        }

        "repeat" => {
            let map = params
                .as_mapping()
                .ok_or_else(|| anyhow::anyhow!("repeat requires a mapping"))?;
            let times = map
                .get(&serde_yaml::Value::String("times".to_string()))
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);

            let while_condition = map
                .get(&serde_yaml::Value::String("while".to_string()))
                .and_then(|v| serde_yaml::from_value(v.clone()).ok());

            let cmds_val = map
                .get(&serde_yaml::Value::String("commands".to_string()))
                .ok_or_else(|| anyhow::anyhow!("repeat requires commands"))?;
            let commands = parse_commands_from_value(cmds_val)?;
            TestCommand::Repeat(RepeatParams {
                times,
                while_condition,
                commands,
            })
        }

        "retry" => {
            let map = params
                .as_mapping()
                .ok_or_else(|| anyhow::anyhow!("retry requires a mapping"))?;
            let max_retries = map
                .get(&serde_yaml::Value::String("maxRetries".to_string()))
                .and_then(|v| v.as_u64())
                .unwrap_or(3) as u32;
            let cmds_val = map
                .get(&serde_yaml::Value::String("commands".to_string()))
                .ok_or_else(|| anyhow::anyhow!("retry requires commands"))?;
            let commands = parse_commands_from_value(cmds_val)?;
            TestCommand::Retry(RetryParams {
                max_retries,
                commands,
            })
        }

        "runFlow" => {
            use super::types::{RunFlowParams, RunFlowParamsInput};
            match params {
                serde_yaml::Value::String(s) => {
                    TestCommand::RunFlow(RunFlowParamsInput::String(s.clone()))
                }
                serde_yaml::Value::Mapping(map) => {
                    let path = map
                        .get(&serde_yaml::Value::String("path".to_string()))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let vars = map
                        .get(&serde_yaml::Value::String("vars".to_string()))
                        .or_else(|| map.get(&serde_yaml::Value::String("env".to_string())))
                        .and_then(|v| serde_yaml::from_value(v.clone()).ok());

                    let commands = if let Some(cmds_val) =
                        map.get(&serde_yaml::Value::String("commands".to_string()))
                    {
                        Some(parse_commands_from_value(cmds_val)?)
                    } else {
                        None
                    };

                    let when = map
                        .get(&serde_yaml::Value::String("when".to_string()))
                        .and_then(|v| serde_yaml::from_value(v.clone()).ok());

                    let label = map
                        .get(&serde_yaml::Value::String("label".to_string()))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let optional = map
                        .get(&serde_yaml::Value::String("optional".to_string()))
                        .and_then(|v| v.as_bool());

                    TestCommand::RunFlow(RunFlowParamsInput::Struct(RunFlowParams {
                        path,
                        vars,
                        commands,
                        when,
                        label,
                        optional,
                    }))
                }
                _ => anyhow::bail!("Invalid runFlow params"),
            }
        }

        "tapAt" => {
            let p: TapAtParams = serde_yaml::from_value(params.clone())?;
            TestCommand::TapAt(p)
        }

        "inputAt" => {
            let p: InputAtParams = serde_yaml::from_value(params.clone())?;
            TestCommand::InputAt(p)
        }

        "setVar" => {
            let p: SetVarParams = serde_yaml::from_value(params.clone())?;
            TestCommand::SetVar(p)
        }

        "assertVar" => {
            let p: AssertVarParams = serde_yaml::from_value(params.clone())?;
            TestCommand::AssertVar(p)
        }

        "screenshot" | "takeScreenshot" => {
            let p: crate::parser::types::ScreenshotParamsInput =
                serde_yaml::from_value(params.clone())?;
            TestCommand::TakeScreenshot(p)
        }

        "startRecording" => {
            let p: crate::parser::types::RecordingParamsInput =
                serde_yaml::from_value(params.clone())?;
            TestCommand::StartRecording(p)
        }

        "exportReport" => {
            let p: ReportParams = serde_yaml::from_value(params.clone())?;
            TestCommand::ExportReport(p)
        }

        "generate" => {
            let p: GenerateParams = serde_yaml::from_value(params.clone())?;
            TestCommand::Generate(p)
        }

        "httpRequest" => {
            let p: HttpRequestParams = serde_yaml::from_value(params.clone())?;
            TestCommand::HttpRequest(p)
        }

        "runScript" => {
            let p: crate::parser::types::RunScriptParamsInput =
                serde_yaml::from_value(params.clone())?;
            TestCommand::RunScript(p)
        }

        "conditional" => {
            let p: ConditionalParams = serde_yaml::from_value(params.clone())?;
            TestCommand::Conditional(p)
        }

        "rightClick" | "contextClick" => {
            let p: TapParams = if params.is_string() {
                let text = params.as_str().unwrap().to_string();
                TapParams {
                    text: Some(text),
                    ..Default::default()
                }
            } else {
                serde_yaml::from_value(params.clone())?
            };
            TestCommand::RightClick(p)
        }

        "back" => TestCommand::Back,
        "stopRecording" | "stopRecord" => TestCommand::StopRecording,
        "stopApp" | "stop" => TestCommand::StopApp,
        "pressHome" | "home" => TestCommand::PressHome,
        "hideKeyboard" | "hideKbd" => TestCommand::HideKeyboard,

        "mockLocation" | "gps" => {
            let p: MockLocationParamsInput = serde_yaml::from_value(params.clone())?;
            TestCommand::MockLocation(p)
        }
        "stopMockLocation" | "stopGps" => TestCommand::StopMockLocation,

        "assertColor" | "checkColor" => {
            let p: AssertColorParams = serde_yaml::from_value(params.clone())?;
            TestCommand::AssertColor(p)
        }

        // GIF Recording
        "captureGifFrame" | "captureFrame" => {
            let p: CaptureGifFrameParamsInput = serde_yaml::from_value(params.clone())?;
            TestCommand::CaptureGifFrame(p)
        }

        "buildGif" | "createGif" => {
            let p: BuildGifParams = serde_yaml::from_value(params.clone())?;
            TestCommand::BuildGif(p)
        }

        "startGifCapture" => {
            let p: crate::parser::types::StartGifCaptureParams =
                serde_yaml::from_value(params.clone())?;
            TestCommand::StartGifCapture(p)
        }

        "stopGifCapture" => {
            let p: crate::parser::types::StopGifCaptureParams =
                serde_yaml::from_value(params.clone())?;
            TestCommand::StopGifCapture(p)
        }

        "rotate" | "rotateScreen" => {
            let p: crate::parser::types::RotationParamsInput =
                serde_yaml::from_value(params.clone())?;
            TestCommand::RotateScreen(p)
        }

        "press" | "pressKey" => {
            let p: crate::parser::types::PressKeyParamsInput =
                serde_yaml::from_value(params.clone())?;
            TestCommand::PressKey(p)
        }

        "pushFile" => {
            let p: crate::parser::types::FileTransferParams =
                serde_yaml::from_value(params.clone())?;
            TestCommand::PushFile(p)
        }

        "pullFile" => {
            let p: crate::parser::types::FileTransferParams =
                serde_yaml::from_value(params.clone())?;
            TestCommand::PullFile(p)
        }

        "waitForLocation" => {
            let p: crate::parser::types::WaitForLocationParams =
                serde_yaml::from_value(params.clone())?;
            TestCommand::WaitForLocation(p)
        }

        "waitForMockCompletion" => {
            let p: crate::parser::types::WaitForMockCompletionParams = if params.is_null() {
                // Handle empty params case e.g. - waitForMockCompletion:
                crate::parser::types::WaitForMockCompletionParams {
                    name: None,
                    timeout: None,
                }
            } else if let Ok(timeout) = serde_yaml::from_value::<u64>(params.clone()) {
                crate::parser::types::WaitForMockCompletionParams {
                    name: None,
                    timeout: Some(timeout),
                }
            } else {
                // Try parsing struct
                serde_yaml::from_value(params.clone()).unwrap_or(
                    crate::parser::types::WaitForMockCompletionParams {
                        name: None,
                        timeout: None,
                    },
                )
            };
            TestCommand::WaitForMockCompletion(p)
        }

        "mockLocationControl" => {
            let p: crate::parser::types::MockLocationControlParams =
                serde_yaml::from_value(params.clone())?;
            TestCommand::MockLocationControl(p)
        }

        "clearAppData" => {
            let pkg = match params {
                serde_yaml::Value::String(s) => s.clone(),
                _ => serde_yaml::from_value(params.clone())?,
            };
            TestCommand::ClearAppData(pkg)
        }

        "setClipboard" => {
            let val = match params {
                serde_yaml::Value::String(s) => s.clone(),
                _ => serde_yaml::from_value(params.clone())?,
            };
            TestCommand::SetClipboard(val)
        }

        "getClipboard" => {
            // Support simple string as variable name
            let p = if params.is_string() {
                crate::parser::types::SetVarParams {
                    name: params.as_str().unwrap().to_string(),
                    value: String::new(),
                }
            } else {
                serde_yaml::from_value(params.clone())?
            };
            TestCommand::GetClipboard(p)
        }

        "assertClipboard" => {
            let val = match params {
                serde_yaml::Value::String(s) => s.clone(),
                _ => serde_yaml::from_value(params.clone())?,
            };
            TestCommand::AssertClipboard(val)
        }

        "assertTrue" | "assert" => {
            let p = if params.is_string() {
                crate::parser::types::AssertTrueParams::Expression(
                    params.as_str().unwrap().to_string(),
                )
            } else {
                serde_yaml::from_value(params.clone())?
            };
            TestCommand::AssertTrue(p)
        }

        "evalScript" => {
            let expr = match params {
                serde_yaml::Value::String(s) => s.clone(),
                _ => serde_yaml::from_value(params.clone())?,
            };
            TestCommand::EvalScript(expr)
        }

        "copyTextFrom" => {
            let p: crate::parser::types::CopyTextFromParams =
                serde_yaml::from_value(params.clone())?;
            TestCommand::CopyTextFrom(p)
        }

        "pasteText" => TestCommand::PasteText,

        "inputRandomEmail" => TestCommand::InputRandomEmail,

        "inputRandomNumber" | "inputRandomPhoneNumber" => {
            let p: Option<crate::parser::types::RandomNumberParams> =
                serde_yaml::from_value(params.clone()).ok();
            TestCommand::InputRandomNumber(p)
        }

        "inputRandomPersonName" => TestCommand::InputRandomPersonName,

        "inputRandomText" => {
            let p: Option<crate::parser::types::RandomTextParams> =
                serde_yaml::from_value(params.clone()).ok();
            TestCommand::InputRandomText(p)
        }

        "extendedWaitUntil" => {
            let p: crate::parser::types::ExtendedWaitParams =
                serde_yaml::from_value(params.clone())?;
            TestCommand::ExtendedWaitUntil(p)
        }

        "setNetwork" => {
            let p: crate::parser::types::NetworkParams = serde_yaml::from_value(params.clone())?;
            TestCommand::SetNetwork(p)
        }

        "airplaneMode" | "toggleAirplaneMode" => TestCommand::ToggleAirplaneMode,

        "openNotifications" => TestCommand::OpenNotifications,
        "openQuickSettings" => TestCommand::OpenQuickSettings,

        "setVolume" => {
            let level = if params.is_number() {
                params.as_u64().unwrap() as u8
            } else {
                serde_yaml::from_value(params.clone())?
            };
            TestCommand::SetVolume(level)
        }

        "lockDevice" => TestCommand::LockDevice,
        "unlockDevice" => TestCommand::UnlockDevice,

        "installApp" => {
            let path = match params {
                serde_yaml::Value::String(s) => s.clone(),
                _ => serde_yaml::from_value(params.clone())?,
            };
            TestCommand::InstallApp(path)
        }

        "uninstallApp" => {
            let pkg = match params {
                serde_yaml::Value::String(s) => s.clone(),
                _ => serde_yaml::from_value(params.clone())?,
            };
            TestCommand::UninstallApp(pkg)
        }

        "backgroundApp" => {
            let p: crate::parser::types::BackgroundAppParams =
                serde_yaml::from_value(params.clone())?;
            TestCommand::BackgroundApp(p)
        }

        "setOrientation" => {
            let p: crate::parser::types::OrientationParams = if params.is_string() {
                // Handle "LANDSCAPE" etc string alias
                let mode: crate::parser::types::Orientation =
                    serde_yaml::from_value(params.clone())?;
                crate::parser::types::OrientationParams { mode }
            } else {
                serde_yaml::from_value(params.clone())?
            };
            TestCommand::SetOrientation(p)
        }

        "dbQuery" => {
            let p: crate::parser::types::DbQueryParams = serde_yaml::from_value(params.clone())?;
            TestCommand::DbQuery(p)
        }

        "openLink" | "deepLink" => {
            let s = match params {
                serde_yaml::Value::String(s) => s.clone(),
                _ => serde_yaml::from_value(params.clone())?,
            };
            TestCommand::OpenLink(s)
        }

        "navigate" => {
            let p = if params.is_string() {
                crate::parser::types::NavigateParams {
                    url: params.as_str().unwrap().to_string(),
                }
            } else {
                serde_yaml::from_value(params.clone())?
            };
            TestCommand::Navigate(p)
        }

        "click" => {
            let p = if params.is_string() {
                crate::parser::types::ClickParams {
                    text: Some(params.as_str().unwrap().to_string()),
                    selector: None,
                }
            } else {
                serde_yaml::from_value(params.clone())?
            };
            TestCommand::Click(p)
        }

        "setLocale" | "locale" => {
            let locale = match params {
                serde_yaml::Value::String(s) => s.clone(),
                _ => serde_yaml::from_value(params.clone())?,
            };
            TestCommand::SetLocale(locale)
        }

        "selectDisplay" | "display" => {
            let id = match params {
                serde_yaml::Value::String(s) => s.clone(),
                serde_yaml::Value::Number(n) => n.to_string(),
                _ => serde_yaml::from_value(params.clone())?,
            };
            TestCommand::SelectDisplay(id)
        }

        _ => return Ok(None),
    };

    Ok(Some(cmd))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_flow() {
        let yaml = r#"
appId: com.example.app
---
- launchApp:
    clearState: true
- tapOn:
    text: "Login"
- inputText: "test@example.com"
- assertVisible:
    text: "Dashboard"
"#;

        let flow = parse_yaml_content(yaml, Path::new("test.yaml")).unwrap();
        assert_eq!(flow.app_id, Some("com.example.app".to_string()));
        assert_eq!(flow.commands.len(), 4);
    }
}
