use crate::driver::traits::PlatformDriver;
use crate::parser::yaml::parse_command_value;
use crate::runner::events::EventEmitter;
use crate::runner::executor::TestExecutor;
use anyhow::Result;
use colored::Colorize;
use std::io::{self, Write};

/// Result of running one shell line: the command's display name (or the raw line, if it
/// never got far enough to parse) plus success/error.
struct LineOutcome {
    display: String,
    success: bool,
    error: Option<String>,
}

/// Parse and execute a single line using the exact same command parser (`parse_command_value`)
/// and `TestExecutor` a real YAML test file's commands go through - a shell command and a
/// YAML test-file command are the same `TestCommand`, just sourced differently, so this must
/// never diverge into a second, parallel notion of "what a command is".
async fn execute_line(executor: &mut TestExecutor, line: &str) -> LineOutcome {
    match serde_yaml::from_str::<serde_yaml::Value>(line) {
        Ok(value) => match parse_command_value(&value) {
            Ok(Some(cmd)) => {
                let display = cmd.display_name();
                match executor.execute_command(&cmd).await {
                    Ok(()) => LineOutcome {
                        display,
                        success: true,
                        error: None,
                    },
                    Err(e) => LineOutcome {
                        display,
                        success: false,
                        error: Some(e.to_string()),
                    },
                }
            }
            Ok(None) => LineOutcome {
                display: line.to_string(),
                success: false,
                error: Some("Unknown command".to_string()),
            },
            Err(e) => LineOutcome {
                display: line.to_string(),
                success: false,
                error: Some(format!("Parse error: {}", e)),
            },
        },
        Err(e) => LineOutcome {
            display: line.to_string(),
            success: false,
            error: Some(format!("YAML error: {}", e)),
        },
    }
}

/// Run one or more commands non-interactively against a single device session, then exit -
/// no stdin loop, no yaml file. This is the form meant for a script or an AI agent to call
/// directly (`lumi-tester shell --device <X> -c 'tapOn: "Login"' -c 'pinch: {direction: open}'`):
/// each `-c` value uses the exact same YAML-sugar syntax as a test file's command list, and
/// all of them share one driver connection (device/app/session setup only happens once,
/// same as it would across an entire YAML test file's steps).
///
/// Returns an error (non-zero process exit, since `main` propagates it) if ANY command
/// failed, so a caller can rely on the exit code alone rather than having to scrape text -
/// `--json` additionally prints one JSON object per line for full machine-readable detail
/// (`{"command":"...","success":bool,"error":string|null}`).
pub async fn run_one_shot(
    driver: Box<dyn PlatformDriver>,
    commands: Vec<String>,
    json_output: bool,
) -> Result<()> {
    let (_emitter, _) = EventEmitter::new();
    let mut executor = TestExecutor::new(driver, None, true, false, false, false, None);

    let mut any_failed = false;

    for raw_line in commands {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let outcome = execute_line(&mut executor, line).await;
        if !outcome.success {
            any_failed = true;
        }

        if json_output {
            let obj = serde_json::json!({
                "command": outcome.display,
                "success": outcome.success,
                "error": outcome.error,
            });
            println!("{}", obj);
        } else if outcome.success {
            println!("{} {}", "✅".green(), outcome.display.cyan());
        } else {
            println!(
                "{} {} - {}",
                "❌".red(),
                outcome.display.cyan(),
                outcome.error.as_deref().unwrap_or("failed")
            );
        }
    }

    if any_failed {
        anyhow::bail!("one or more commands failed");
    }
    Ok(())
}

pub async fn run_shell(driver: Box<dyn PlatformDriver>) -> Result<()> {
    let (_emitter, _) = EventEmitter::new();
    let mut executor = TestExecutor::new(driver, None, true, false, false, false, None);

    println!(
        "\n{}",
        "=== lumi-tester Interactive Shell ===".bold().green()
    );
    println!(
        "Type commands (e.g., 'tap \"Settings\"', 'back', 'see \"Display\"') or 'exit' to quit."
    );
    println!("Tip: You can use the same sugar syntax as in YAML test files.\n");

    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print!("{} ", "lumi-tester>".blue().bold());
        io::stdout().flush().unwrap();

        input.clear();
        if stdin.read_line(&mut input)? == 0 {
            break; // EOF
        }

        let line = input.trim();
        if line.is_empty() {
            continue;
        }

        if line == "exit" || line == "quit" {
            break;
        }

        let outcome = execute_line(&mut executor, line).await;
        if outcome.success {
            println!("{} {} - passed", "✅".green(), outcome.display.cyan());
        } else {
            println!(
                "{} {} - {}",
                "❌".red(),
                outcome.display.cyan(),
                outcome.error.as_deref().unwrap_or("failed")
            );
        }
    }

    println!("\nExiting shell. Goodbye!");
    Ok(())
}
