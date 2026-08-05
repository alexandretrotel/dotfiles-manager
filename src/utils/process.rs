use std::path::Path;
use std::process::Command;

use crate::error::Error;
use crate::error::Result;

/// Runs `cmd` with `args` (optionally in `dir`), returns stdout or `Error::CommandFailure`.
pub(crate) fn run_cmd(cmd: &str, args: &[&str], dir: Option<&Path>) -> Result<String> {
    let mut command = Command::new(cmd);
    command.args(args);

    if let Some(d) = dir {
        command.current_dir(d);
    }

    let output = command.output()?;

    if !output.status.success() {
        let stderr_len = output.stderr.len();
        let stderr_msg = String::from_utf8(output.stderr)
            .unwrap_or_else(|_| format!("<non-UTF-8 stderr data: {} bytes>", stderr_len));

        return Err(Error::CommandFailure {
            cmd: cmd.to_string(),
            status: output.status.code(),
            stderr: stderr_msg,
        });
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout)
}

/// Checks `command` exists: as a file path (if it has multiple components) or on `PATH`.
pub(crate) fn is_command_available(command: &str) -> bool {
    let command_path = Path::new(command);
    if command_path.components().count() > 1 {
        return command_path.is_file();
    }

    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };

    for path_dir in std::env::split_paths(&path_var) {
        let candidate = path_dir.join(command);
        if candidate.is_file() {
            return true;
        }
    }

    false
}
