mod config;
mod daemon;
mod ipc;
mod provider;
mod providers;
mod usage;

use std::{
    io,
    path::PathBuf,
};

use crate::daemon::Daemon;

fn main() -> anyhow::Result<()> {
    let socket_path = socket_path().expect("socket path should resolve");

    let daemon = Daemon::new(&socket_path)?;
    daemon.run();

    Ok(())
}

fn socket_path() -> io::Result<PathBuf> {
    if let Ok(socket_path) = std::env::var("RIKA_LAUNCHER_SOCKET") {
        return Ok(PathBuf::from(socket_path));
    }

    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .map_err(|err| io::Error::new(io::ErrorKind::NotFound, err))?;

    Ok(PathBuf::from(runtime_dir).join("rika-launcher.sock"))
}
