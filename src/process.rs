use std::{
    io::Write,
    process::{
        Command,
        Stdio,
    },
    sync::mpsc,
    thread,
};

use anyhow::{
    Context,
    anyhow,
};

/// Spawn and reap a child on one dedicated thread.
///
/// Some sandbox launchers tie their parent-death signal to the thread that
/// calls `Command::spawn`, so that thread must live until the child exits.
pub fn spawn(command: Command, description: impl Into<String>) -> anyhow::Result<()> {
    spawn_inner(command, None, description.into())
}

pub fn spawn_with_stdin(
    mut command: Command,
    input: Vec<u8>,
    description: impl Into<String>,
) -> anyhow::Result<()> {
    command.stdin(Stdio::piped());
    spawn_inner(command, Some(input), description.into())
}

fn spawn_inner(
    mut command: Command,
    input: Option<Vec<u8>>,
    description: String,
) -> anyhow::Result<()> {
    let (started_tx, started_rx) = mpsc::sync_channel(0);
    let thread_description = description.clone();

    thread::Builder::new()
        .name("rika-child".to_string())
        .spawn(move || {
            let mut child = match command.spawn() {
                Ok(child) => child,
                Err(err) => {
                    let _ = started_tx.send(Err(err.into()));
                    return;
                }
            };

            if let Some(input) = input {
                let write_result = child
                    .stdin
                    .take()
                    .context("while attempting to open child stdin")
                    .and_then(|mut stdin| {
                        stdin
                            .write_all(&input)
                            .context("while attempting to write child stdin")
                    });

                if let Err(err) = write_result {
                    let _ = started_tx.send(Err(err));
                    if let Err(wait_err) = child.wait() {
                        eprintln!("failed to reap {thread_description}: {wait_err}");
                    }
                    return;
                }
            }

            let _ = started_tx.send(Ok(()));

            match child.wait() {
                Ok(status) if status.success() => {}
                Ok(status) => eprintln!("{thread_description} exited with {status}"),
                Err(err) => eprintln!("failed to reap {thread_description}: {err}"),
            }
        })
        .with_context(|| format!("while attempting to start {description} owner thread"))?;

    started_rx
        .recv()
        .map_err(|_| anyhow!("{description} owner thread exited before reporting startup"))?
        .with_context(|| format!("while attempting to spawn {description}"))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        process::Command,
        thread,
        time::{
            Duration,
            Instant,
            SystemTime,
            UNIX_EPOCH,
        },
    };

    use super::*;

    fn temp_file(test_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("rika-process-{test_name}-{nanos}"))
    }

    fn wait_for_file(path: &PathBuf) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !path.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(path.exists(), "child did not create {}", path.display());
    }

    #[test]
    fn reports_spawn_failures_to_the_caller() {
        let command = Command::new("/rika-command-that-does-not-exist");
        let err = spawn(command, "missing test command").expect_err("spawn should fail");

        assert!(
            err.to_string()
                .contains("while attempting to spawn missing test command")
        );
    }

    #[test]
    fn writes_stdin_before_returning_success() {
        let output = temp_file("stdin");
        let mut command = Command::new("sh");
        command.args(["-c", "cat > \"$1\"", "sh"]);
        command.arg(&output);

        spawn_with_stdin(
            command,
            b"rika clipboard test".to_vec(),
            "stdin test command",
        )
        .expect("command should start and accept input");

        wait_for_file(&output);
        assert_eq!(
            fs::read(&output).expect("output should be readable"),
            b"rika clipboard test"
        );
        let _ = fs::remove_file(output);
    }

    #[test]
    fn returns_before_the_child_exits() {
        let release = temp_file("release");
        let completed = temp_file("completed");
        let mut command = Command::new("sh");
        command.args([
            "-c",
            "while [ ! -e \"$1\" ]; do sleep 0.01; done; touch \"$2\"",
            "sh",
        ]);
        command.arg(&release).arg(&completed);

        spawn(command, "waiting test command").expect("command should start");
        assert!(!completed.exists(), "child should still be running");

        fs::write(&release, "").expect("release file should be created");
        wait_for_file(&completed);
        let _ = fs::remove_file(release);
        let _ = fs::remove_file(completed);
    }
}
