use std::process::Command;

use crate::process;

pub fn copy_text(text: &str) -> anyhow::Result<()> {
    let command = Command::new("wl-copy");
    process::spawn_with_stdin(command, text.as_bytes().to_vec(), "wl-copy")
}
