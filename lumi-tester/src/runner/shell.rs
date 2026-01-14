use crate::driver::traits::PlatformDriver;
use crate::parser::yaml::parse_command_value;
use crate::runner::events::EventEmitter;
use crate::runner::executor::TestExecutor;
use anyhow::Result;
use colored::Colorize;
use std::io::{self, Write};

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

        // Try to parse the line as a YAML-style command
        // We'll try to wrap it if it doesn't look like a YAML mapping
        let yaml_input = if line.contains(':') {
            line.to_string()
        } else {
            line.to_string() // parse_command_value handles simple strings
        };

        match serde_yaml::from_str::<serde_yaml::Value>(&yaml_input) {
            Ok(value) => match parse_command_value(&value) {
                Ok(Some(cmd)) => {
                    println!("{} Executing: {}", "▶".green(), cmd.display_name().cyan());
                    if let Err(e) = executor.execute_command(&cmd).await {
                        println!("{} Error: {}", "❌".red(), e);
                    } else {
                        println!("{} Command passed.", "✅".green());
                    }
                }
                Ok(None) => {
                    println!("{} Unknown command: {}", "⚠".yellow(), line);
                }
                Err(e) => {
                    println!("{} Parse error: {}", "❌".red(), e);
                }
            },
            Err(e) => {
                println!("{} YAML error: {}", "❌".red(), e);
            }
        }
    }

    println!("\nExiting shell. Goodbye!");
    Ok(())
}
