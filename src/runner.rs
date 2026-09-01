//! Task execution with least privilege and secret handling.
//!
//! Helper scripts run as the invoking user. Privileged work stays inside the
//! scripts' own `sudo` calls after this process caches credentials with
//! `sudo -S -v`. The sudo password is never written to disk, never placed in
//! argv/env, and is redacted from any captured output before it reaches the UI.

use std::{
    io::Write,
    process::{Command, Stdio},
    sync::mpsc::Sender,
};

use crate::catalog::{Task, display_command};
use crate::validate::{is_safe_script_name, redact_secret, zeroize_string};

#[derive(Debug)]
pub enum RunnerMessage {
    Log(String),
    Done,
}

pub fn run_tasks(tasks: Vec<Task>, mut password: String, tx: Sender<RunnerMessage>) {
    if let Err(error) = cache_sudo_credentials(&password) {
        let _ = tx.send(RunnerMessage::Log(redact_secret(&error, &password)));
        zeroize_string(&mut password);
        let _ = tx.send(RunnerMessage::Done);
        return;
    }

    for task in tasks {
        let _ = tx.send(RunnerMessage::Log(format!(
            "[INFO] Starting: {}",
            task.description
        )));

        if let Err(error) = validate_task_command(&task.command) {
            let _ = tx.send(RunnerMessage::Log(format!("[ERROR] {error}")));
            continue;
        }

        let _ = tx.send(RunnerMessage::Log(format!(
            "Command: {}",
            display_command(&task.command)
        )));

        let Some((program, args)) = task.command.split_first() else {
            let _ = tx.send(RunnerMessage::Log("[ERROR] Empty command.".to_owned()));
            continue;
        };

        let output = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .and_then(|child| child.wait_with_output());

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                for line in stdout.lines().chain(stderr.lines()) {
                    let _ = tx.send(RunnerMessage::Log(redact_secret(line, &password)));
                }
                if output.status.success() {
                    let _ = tx.send(RunnerMessage::Log(format!(
                        "[SUCCESS] {} completed successfully.",
                        task.description
                    )));
                } else {
                    let code = output
                        .status
                        .code()
                        .map_or_else(|| "unknown".to_owned(), |code| code.to_string());
                    let _ = tx.send(RunnerMessage::Log(format!(
                        "[ERROR] {} failed with return code {code}.",
                        task.description
                    )));
                }
            }
            Err(error) => {
                let _ = tx.send(RunnerMessage::Log(redact_secret(
                    &format!("[EXCEPTION] Failed to run {}: {error}", task.description),
                    &password,
                )));
            }
        }
    }

    drop_sudo_credentials();
    zeroize_string(&mut password);
    let _ = tx.send(RunnerMessage::Done);
}

fn validate_task_command(command: &[String]) -> Result<(), String> {
    let Some((program, args)) = command.split_first() else {
        return Err("Empty command.".to_owned());
    };
    if program != "bash" {
        return Err("Only bash helper scripts are allowed.".to_owned());
    }
    let Some(script) = args.first() else {
        return Err("Missing helper script path.".to_owned());
    };
    let name = std::path::Path::new(script)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Invalid helper script path.".to_owned())?;
    if !is_safe_script_name(name) {
        return Err(format!("Helper script {name} is not allowed."));
    }
    if args.len() > 1 && name != "main.sh" {
        return Err("Admin helper scripts do not accept extra arguments.".to_owned());
    }
    Ok(())
}

fn cache_sudo_credentials(password: &str) -> Result<(), String> {
    if password.is_empty() {
        return Err("[ERROR] A sudo password is required.".to_owned());
    }

    let mut child = Command::new("sudo")
        .args(["-S", "-v"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "[ERROR] Failed to start sudo.".to_owned())?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(password.as_bytes())
            .map_err(|_| "[ERROR] Failed to submit sudo credentials.".to_owned())?;
        stdin
            .write_all(b"\n")
            .map_err(|_| "[ERROR] Failed to submit sudo credentials.".to_owned())?;
    }

    let output = child
        .wait_with_output()
        .map_err(|_| "[ERROR] Failed to wait for sudo.".to_owned())?;

    if output.status.success() {
        Ok(())
    } else {
        Err("[ERROR] sudo authentication failed.".to_owned())
    }
}

fn drop_sudo_credentials() {
    let _ = Command::new("sudo")
        .arg("-k")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_task_command_accepts_main_sh_argv() {
        assert!(
            validate_task_command(&[
                "bash".into(),
                "/opt/toolbox/main.sh".into(),
                "--label".into(),
                "Firefox".into(),
                "--package".into(),
                "firefox".into(),
                "install".into(),
            ])
            .is_ok()
        );
    }

    #[test]
    fn validate_task_command_accepts_admin_script_without_args() {
        assert!(
            validate_task_command(&["bash".into(), "/opt/toolbox/update-system.sh".into()]).is_ok()
        );
    }

    #[test]
    fn validate_task_command_rejects_shell_or_unknown_binaries() {
        assert!(validate_task_command(&["sh".into(), "-c".into(), "id".into()]).is_err());
        assert!(validate_task_command(&["sudo".into(), "pacman".into(), "-Syu".into()]).is_err());
        assert!(validate_task_command(&["bash".into(), "/tmp/evil.sh".into()]).is_err());
        assert!(
            validate_task_command(&[
                "bash".into(),
                "/opt/toolbox/update-system.sh".into(),
                "extra".into()
            ])
            .is_err()
        );
    }

    #[test]
    fn cache_sudo_requires_password() {
        assert!(cache_sudo_credentials("").is_err());
    }
}
