use std::{
    io::Write,
    process::{
        Child,
        Command,
        Stdio,
    },
    thread,
};

use anyhow::Context;

pub fn copy_text(text: &str) -> anyhow::Result<()> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .context("while attempting to spawn wl-copy")?;

    let mut stdin = child
        .stdin
        .take()
        .context("while attempting to open wl-copy stdin")?;
    stdin
        .write_all(text.as_bytes())
        .context("while attempting to write clipboard text")?;
    drop(stdin);

    reap_child(child);

    Ok(())
}

fn reap_child(mut child: Child) {
    thread::spawn(move || match child.wait() {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("wl-copy exited with {status}"),
        Err(err) => eprintln!("failed to wait for wl-copy: {err}"),
    });
}
