//! Helpers for launching background processes without flashing console windows.

#[cfg(any(windows, test))]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(any(windows, test))]
fn background_process_creation_flags(existing_flags: u32) -> u32 {
    existing_flags | CREATE_NO_WINDOW
}

/// Configure a standard process command to hide a console window on Windows.
pub fn configure_hidden_std_command(command: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        command.creation_flags(background_process_creation_flags(0));
    }

    #[cfg(not(windows))]
    let _ = command;
}

/// Configure an async process command to hide a console window on Windows.
pub fn configure_hidden_tokio_command(command: &mut tokio::process::Command) {
    #[cfg(windows)]
    configure_hidden_std_command(command.as_std_mut());

    #[cfg(not(windows))]
    let _ = command;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_process_flags_hide_console_and_preserve_existing_flags() {
        assert_eq!(
            background_process_creation_flags(0x00000004),
            0x08000004
        );
    }
}
