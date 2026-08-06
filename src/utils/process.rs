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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonexistent_command_returns_error() {
        let result = run_cmd("this-command-definitely-does-not-exist-xyz", &[], None);
        assert!(result.is_err());
    }

    #[test]
    fn is_command_available_false_for_nonexistent_command() {
        assert!(!is_command_available(
            "this-command-definitely-does-not-exist-xyz"
        ));
    }

    #[test]
    fn is_command_available_checks_multi_component_path_directly() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("some-file");
        std::fs::write(&file, b"").unwrap();

        assert!(is_command_available(file.to_str().unwrap()));

        let missing = dir.path().join("missing-file");
        assert!(!is_command_available(missing.to_str().unwrap()));
    }

    #[cfg(unix)]
    #[test]
    fn run_cmd_captures_stdout() {
        let output = run_cmd("echo", &["hello"], None).unwrap();
        assert_eq!(output, "hello\n");
    }

    #[cfg(unix)]
    #[test]
    fn run_cmd_runs_in_given_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("marker.txt"), b"").unwrap();

        let output = run_cmd("ls", &[], Some(dir.path())).unwrap();
        assert!(output.contains("marker.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn run_cmd_returns_command_failure_on_nonzero_exit() {
        let result = run_cmd("false", &[], None);
        match result {
            Err(Error::CommandFailure { cmd, .. }) => assert_eq!(cmd, "false"),
            other => panic!("expected CommandFailure, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn is_command_available_true_for_known_command() {
        assert!(is_command_available("echo"));
    }
}
