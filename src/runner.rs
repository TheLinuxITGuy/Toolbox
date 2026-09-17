//! Task execution with least privilege and secret handling.
//!
//! Helper scripts run as the invoking user. Privileged work stays inside the
//! scripts' own `sudo` calls after this process caches credentials with
//! `sudo -S -v`. The sudo password is never written to disk, never placed in
//! argv/env, and is redacted from any captured output before it reaches the UI.

use std::{
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::mpsc::Sender,
};

use crate::catalog::{Task, display_command, helper_path_is_under_base};
use crate::system::{command_exists, detect_package_manager};
use crate::validate::{
    is_flatpak_id, is_safe_script_name, redact_secret, validate_main_sh_argv,
    validate_package_name, zeroize_string,
};

#[derive(Debug)]
pub enum RunnerMessage {
    Log(String),
    Done,
}

pub fn run_tasks(
    tasks: Vec<Task>,
    mut password: String,
    tx: Sender<RunnerMessage>,
    base_dir: PathBuf,
) {
    let needs_sudo = tasks_need_privileges(&tasks);
    if needs_sudo {
        if let Err(error) = cache_sudo_credentials(&password) {
            let _ = tx.send(RunnerMessage::Log(redact_secret(&error, &password)));
            zeroize_string(&mut password);
            let _ = tx.send(RunnerMessage::Done);
            return;
        }
    } else {
        let _ = tx.send(RunnerMessage::Log(
            "[INFO] Skipping sudo; selected tasks run as the current user.".to_owned(),
        ));
        zeroize_string(&mut password);
    }

    if tasks.iter().any(|task| task_needs_flatpak(&task.command))
        && let Err(error) = ensure_flatpak_package(&password, &tx)
    {
        let _ = tx.send(RunnerMessage::Log(redact_secret(&error, &password)));
    }

    for task in tasks {
        let _ = tx.send(RunnerMessage::Log(format!(
            "[INFO] Starting: {}",
            task.description
        )));

        if let Err(error) = validate_task_command(&task.command, &base_dir) {
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

        let output = spawn_captured(program, args);

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
                    &format!(
                        "[EXCEPTION] Failed to run {}: {}",
                        task.description,
                        io_error_message(program, &error)
                    ),
                    &password,
                )));
            }
        }
    }

    if needs_sudo {
        drop_sudo_credentials();
    }
    zeroize_string(&mut password);
    let _ = tx.send(RunnerMessage::Done);
}

fn spawn_captured(program: &str, args: &[String]) -> Result<Output, std::io::Error> {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|child| child.wait_with_output())
}

fn io_error_message(program: &str, error: &std::io::Error) -> String {
    if error.kind() == ErrorKind::NotFound {
        format!("{program} was not found on PATH.")
    } else {
        error.to_string()
    }
}

pub fn task_needs_flatpak(command: &[String]) -> bool {
    let Some((_, args)) = command.split_first() else {
        return false;
    };
    let Some(script) = args.first() else {
        return false;
    };
    let name = Path::new(script)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    name == "main.sh"
        && args
            .windows(2)
            .any(|pair| pair[0] == "--flatpak" && is_flatpak_id(&pair[1]))
}

/// Admin recipes and native package installs need sudo. User Flatpak
/// install/remove does not, once the `flatpak` binary is already present.
pub fn command_needs_privileges(command: &[String]) -> bool {
    let Some((program, args)) = command.split_first() else {
        return true;
    };
    if program != "bash" {
        return true;
    }
    let Some(script) = args.first() else {
        return true;
    };
    let name = Path::new(script)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if name != "main.sh" {
        return true;
    }
    if args.windows(2).any(|pair| pair[0] == "--package") {
        return true;
    }
    let has_flatpak = args
        .windows(2)
        .any(|pair| pair[0] == "--flatpak" && is_flatpak_id(&pair[1]));
    if has_flatpak {
        return !command_exists("flatpak");
    }
    true
}

pub fn tasks_need_privileges(tasks: &[Task]) -> bool {
    tasks
        .iter()
        .any(|task| command_needs_privileges(&task.command))
}

pub fn flatpak_package_install_command(package_manager: &str) -> Result<Vec<String>, String> {
    validate_package_name("flatpak").map_err(|error| error.to_string())?;

    let mut command = vec!["sudo".to_owned(), "-S".to_owned()];
    match package_manager {
        "pacman" => command.extend([
            "pacman".to_owned(),
            "-S".to_owned(),
            "--needed".to_owned(),
            "--noconfirm".to_owned(),
            "--".to_owned(),
            "flatpak".to_owned(),
        ]),
        "apt-get" | "apt" => {
            if command_exists("nala") {
                command.extend([
                    "nala".to_owned(),
                    "install".to_owned(),
                    "-y".to_owned(),
                    "--".to_owned(),
                    "flatpak".to_owned(),
                ]);
            } else {
                command.extend([
                    "apt-get".to_owned(),
                    "install".to_owned(),
                    "-y".to_owned(),
                    "--".to_owned(),
                    "flatpak".to_owned(),
                ]);
            }
        }
        "dnf" => command.extend([
            "dnf".to_owned(),
            "install".to_owned(),
            "-y".to_owned(),
            "--".to_owned(),
            "flatpak".to_owned(),
        ]),
        other => {
            return Err(format!(
                "Unsupported package manager for Flatpak bootstrap: {other}"
            ));
        }
    }
    Ok(command)
}

fn ensure_flatpak_package(password: &str, tx: &Sender<RunnerMessage>) -> Result<(), String> {
    if command_exists("flatpak") {
        let _ = tx.send(RunnerMessage::Log(
            "[INFO] Flatpak is already installed.".to_owned(),
        ));
        return Ok(());
    }

    let _ = tx.send(RunnerMessage::Log(
        "[INFO] Flatpak is not installed. Installing the flatpak package...".to_owned(),
    ));

    let package_manager = detect_package_manager();
    let command = flatpak_package_install_command(&package_manager)?;
    let _ = tx.send(RunnerMessage::Log(format!(
        "Command: {}",
        display_command(&command)
    )));

    let output = run_sudo_command(&command, password)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    for line in stdout.lines().chain(stderr.lines()) {
        let _ = tx.send(RunnerMessage::Log(redact_secret(line, password)));
    }

    if !output.status.success() {
        return Err("[ERROR] Failed to install the flatpak package.".to_owned());
    }
    if !command_exists("flatpak") {
        return Err(
            "[ERROR] The flatpak command is still missing after package install.".to_owned(),
        );
    }

    let _ = tx.send(RunnerMessage::Log(
        "[SUCCESS] Flatpak package installed.".to_owned(),
    ));
    Ok(())
}

fn run_sudo_command(command: &[String], password: &str) -> Result<Output, String> {
    let Some((program, args)) = command.split_first() else {
        return Err("[ERROR] Empty Flatpak bootstrap command.".to_owned());
    };
    if program != "sudo" || args.first().is_none_or(|arg| arg != "-S") {
        return Err("[ERROR] Flatpak bootstrap must use sudo -S.".to_owned());
    }

    let mut child = match Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(format!("[ERROR] {program} was not found on PATH."));
        }
        Err(error) => {
            return Err(format!("[ERROR] Failed to start {program}: {error}"));
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(password.as_bytes())
            .and_then(|()| stdin.write_all(b"\n"))
            .map_err(|_| "[ERROR] Failed to submit sudo credentials.".to_owned())?;
    }

    child
        .wait_with_output()
        .map_err(|error| format!("[ERROR] {}", io_error_message(program, &error)))
}

fn validate_task_command(command: &[String], base_dir: &Path) -> Result<(), String> {
    let Some((program, args)) = command.split_first() else {
        return Err("Empty command.".to_owned());
    };
    if program != "bash" {
        return Err("Only bash helper scripts are allowed.".to_owned());
    }
    let Some(script) = args.first() else {
        return Err("Missing helper script path.".to_owned());
    };
    let canonical = helper_path_is_under_base(Path::new(script), base_dir)?;
    let name = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Invalid helper script path.".to_owned())?;
    if !is_safe_script_name(name) {
        return Err(format!("Helper script {name} is not allowed."));
    }
    let extra = &args[1..];
    if name == "main.sh" {
        validate_main_sh_argv(extra).map_err(|error| error.to_string())?;
    } else if !extra.is_empty() {
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
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempToolbox {
        path: PathBuf,
    }

    impl Drop for TempToolbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn temp_toolbox() -> TempToolbox {
        let seq = TEST_DIR_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "linux-it-guy-toolbox-runner-{}-{}",
            std::process::id(),
            seq
        ));
        fs::create_dir_all(&path).expect("create temp toolbox dir");
        fs::write(path.join("main.sh"), "#!/bin/bash\n").unwrap();
        fs::write(path.join("update-system.sh"), "#!/bin/bash\n").unwrap();
        TempToolbox { path }
    }

    fn main_sh_install(base: &Path) -> Vec<String> {
        vec![
            "bash".into(),
            base.join("main.sh").to_string_lossy().into_owned(),
            "--label".into(),
            "Firefox".into(),
            "--package".into(),
            "firefox".into(),
            "install".into(),
        ]
    }

    #[test]
    fn validate_task_command_accepts_main_sh_argv() {
        let toolbox = temp_toolbox();
        assert!(validate_task_command(&main_sh_install(&toolbox.path), &toolbox.path).is_ok());
    }

    #[test]
    fn validate_task_command_accepts_admin_script_without_args() {
        let toolbox = temp_toolbox();
        let script = toolbox
            .path
            .join("update-system.sh")
            .to_string_lossy()
            .into_owned();
        assert!(validate_task_command(&["bash".into(), script], &toolbox.path).is_ok());
    }

    #[test]
    fn validate_task_command_rejects_shell_or_unknown_binaries() {
        let toolbox = temp_toolbox();
        let update = toolbox
            .path
            .join("update-system.sh")
            .to_string_lossy()
            .into_owned();
        assert!(
            validate_task_command(&["sh".into(), "-c".into(), "id".into()], &toolbox.path).is_err()
        );
        assert!(
            validate_task_command(
                &["sudo".into(), "pacman".into(), "-Syu".into()],
                &toolbox.path
            )
            .is_err()
        );
        assert!(
            validate_task_command(&["bash".into(), "/tmp/evil.sh".into()], &toolbox.path).is_err()
        );
        assert!(
            validate_task_command(&["bash".into(), update, "extra".into()], &toolbox.path).is_err()
        );
    }

    #[test]
    fn validate_task_command_rejects_allowlisted_basename_outside_base() {
        let toolbox = temp_toolbox();
        let outsider = std::env::temp_dir().join(format!(
            "linux-it-guy-toolbox-evil-{}-{}",
            std::process::id(),
            TEST_DIR_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&outsider).unwrap();
        let script = outsider.join("main.sh");
        fs::write(&script, "#!/bin/bash\necho pwned\n").unwrap();
        let command = vec![
            "bash".into(),
            script.to_string_lossy().into_owned(),
            "--label".into(),
            "Firefox".into(),
            "--package".into(),
            "firefox".into(),
            "install".into(),
        ];
        assert!(validate_task_command(&command, &toolbox.path).is_err());
        let _ = fs::remove_dir_all(outsider);
    }

    #[test]
    fn validate_task_command_rejects_invalid_main_sh_payloads() {
        let toolbox = temp_toolbox();
        let script = toolbox.path.join("main.sh").to_string_lossy().into_owned();
        assert!(
            validate_task_command(
                &[
                    "bash".into(),
                    script.clone(),
                    "--label".into(),
                    "Firefox".into(),
                    "--package".into(),
                    "-Syu".into(),
                    "install".into(),
                ],
                &toolbox.path
            )
            .is_err()
        );
        assert!(
            validate_task_command(
                &[
                    "bash".into(),
                    script,
                    "--label".into(),
                    "Firefox".into(),
                    "--package".into(),
                    "firefox".into(),
                    "--extra".into(),
                    "x".into(),
                    "install".into(),
                ],
                &toolbox.path
            )
            .is_err()
        );
    }

    #[test]
    fn cache_sudo_requires_password() {
        assert!(cache_sudo_credentials("").is_err());
    }

    #[test]
    fn native_and_admin_tasks_need_privileges_flatpak_user_does_not() {
        let toolbox = temp_toolbox();
        let native = main_sh_install(&toolbox.path);
        assert!(command_needs_privileges(&native));

        let admin = vec![
            "bash".to_owned(),
            toolbox
                .path
                .join("update-system.sh")
                .to_string_lossy()
                .into_owned(),
        ];
        assert!(command_needs_privileges(&admin));

        let flatpak = vec![
            "bash".into(),
            toolbox.path.join("main.sh").to_string_lossy().into_owned(),
            "--label".into(),
            "Brave Browser".into(),
            "--flatpak".into(),
            "com.brave.Browser".into(),
            "install".into(),
        ];
        if command_exists("flatpak") {
            assert!(!command_needs_privileges(&flatpak));
        } else {
            assert!(command_needs_privileges(&flatpak));
        }
        assert!(tasks_need_privileges(&[Task {
            description: "Installing Firefox".into(),
            command: native,
        }]));
    }

    #[test]
    fn task_needs_flatpak_only_for_validated_flatpak_argv() {
        assert!(task_needs_flatpak(&[
            "bash".into(),
            "/opt/toolbox/main.sh".into(),
            "--label".into(),
            "Brave Browser".into(),
            "--flatpak".into(),
            "com.brave.Browser".into(),
            "install".into(),
        ]));
        assert!(!task_needs_flatpak(&[
            "bash".into(),
            "/opt/toolbox/main.sh".into(),
            "--label".into(),
            "Firefox".into(),
            "--package".into(),
            "firefox".into(),
            "install".into(),
        ]));
        assert!(!task_needs_flatpak(&[
            "bash".into(),
            "/opt/toolbox/update-system.sh".into()
        ]));
        assert!(!task_needs_flatpak(&[
            "bash".into(),
            "/opt/toolbox/main.sh".into(),
            "--flatpak".into(),
            "not a valid id".into(),
            "install".into(),
        ]));
    }

    #[test]
    fn flatpak_bootstrap_argv_uses_sudo_s_and_validated_package() {
        let pacman = flatpak_package_install_command("pacman").unwrap();
        assert_eq!(
            pacman,
            [
                "sudo",
                "-S",
                "pacman",
                "-S",
                "--needed",
                "--noconfirm",
                "--",
                "flatpak"
            ]
        );

        let apt = flatpak_package_install_command("apt-get").unwrap();
        assert_eq!(apt[0], "sudo");
        assert_eq!(apt[1], "-S");
        assert!(apt.contains(&"flatpak".to_owned()));
        assert!(apt.contains(&"--".to_owned()));
        assert!(apt.contains(&"apt-get".to_owned()) || apt.contains(&"nala".to_owned()));

        let dnf = flatpak_package_install_command("dnf").unwrap();
        assert_eq!(dnf, ["sudo", "-S", "dnf", "install", "-y", "--", "flatpak"]);

        assert!(flatpak_package_install_command("unknown").is_err());
        assert!(flatpak_package_install_command("pacman;id").is_err());
    }

    #[test]
    fn missing_flatpak_binary_is_enoent_not_a_hang() {
        let error = Command::new("flatpak")
            .env("PATH", "/var/empty-toolbox-no-bin")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect_err("empty PATH must not resolve flatpak");
        assert_eq!(error.kind(), ErrorKind::NotFound);
        assert!(io_error_message("flatpak", &error).contains("was not found on PATH"));
    }

    #[test]
    fn run_sudo_command_rejects_non_sudo_s() {
        assert!(run_sudo_command(&["pacman".into(), "-S".into(), "flatpak".into()], "x").is_err());
        assert!(run_sudo_command(&["sudo".into(), "pacman".into(), "-S".into()], "x").is_err());
    }
}
