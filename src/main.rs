mod provider;

use std::{
    io::{
        self,
        BufRead,
        BufReader,
        Write,
    },
    os::unix::net::{
        UnixListener,
        UnixStream,
    },
    path::PathBuf,
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::provider::{
    MockProvider,
    Provider,
    SearchResult,
};

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ClientRequest {
    #[serde(rename = "query")]
    Query { request_id: u64, query: String },

    #[serde(rename = "activate")]
    Activate {
        provider: String,
        id: String,
        action: String,
    },
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ServerResponse {
    #[serde(rename = "results")]
    Results {
        request_id: u64,
        items: Vec<SearchResult>,
    },

    #[serde(rename = "activated")]
    Activated {
        provider: String,
        id: String,
        action: String,
    },
}

fn main() {
    let socket_path = socket_path().expect("socket path should resolve");

    if let Err(err) = remove_stale_socket(&socket_path) {
        eprintln!(
            "failed to remove stale socket at {}: {err}",
            socket_path.display()
        );
        return;
    }

    let listener = match UnixListener::bind(&socket_path) {
        Ok(sock) => sock,
        Err(e) => {
            eprintln!("failed to bind socket at {}: {e}", socket_path.display());
            return;
        }
    };

    let mock_provider = MockProvider::new();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => match handle_client_request(&mut stream, &mock_provider) {
                Ok(_) => println!("sent client request"),
                Err(err) => eprintln!("failed to handle request: {err}"),
            },
            Err(err) => {
                eprintln!("failed to accept connection: {err}");
            }
        }
    }
}

fn handle_client_request(stream: &mut UnixStream, provider: &MockProvider) -> anyhow::Result<()> {
    let reader_stream = stream.try_clone()?;
    let mut reader = BufReader::new(reader_stream);
    let mut line = String::new();

    loop {
        line.clear();

        if reader.read_line(&mut line)? == 0 {
            break;
        }

        let request: ClientRequest = serde_json::from_str(&line)?;
        match request {
            ClientRequest::Query { request_id, query } => {
                let results = provider.search(&query);

                let response = ServerResponse::Results {
                    request_id,
                    items: results,
                };

                write_response(stream, &response)?;
            }
            ClientRequest::Activate {
                provider,
                id,
                action,
            } => {
                let response = ServerResponse::Activated {
                    provider,
                    id,
                    action,
                };

                write_response(stream, &response)?;
            }
        }
    }

    Ok(())
}

fn write_response(stream: &mut impl Write, response: &ServerResponse) -> io::Result<()> {
    serde_json::to_writer(&mut *stream, response)?;
    stream.write_all(b"\n")?;
    stream.flush()
}

fn socket_path() -> io::Result<PathBuf> {
    if let Ok(socket_path) = std::env::var("RIKA_LAUNCHER_SOCKET") {
        return Ok(PathBuf::from(socket_path));
    }

    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .map_err(|err| io::Error::new(io::ErrorKind::NotFound, err))?;

    Ok(PathBuf::from(runtime_dir).join("rika-launcher.sock"))
}

fn remove_stale_socket(socket_path: &PathBuf) -> io::Result<()> {
    match std::fs::remove_file(socket_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}
