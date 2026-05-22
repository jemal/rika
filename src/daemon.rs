use std::{
    io::{
        self,
        BufRead,
        BufReader,
    },
    os::unix::net::{
        UnixListener,
        UnixStream,
    },
    path::PathBuf,
    sync::{
        Arc,
        Mutex,
    },
    thread,
};

use anyhow::{
    Context,
    bail,
};

use crate::{
    config::Config,
    io::Write,
    ipc::{
        ClientRequest,
        ServerResponse,
    },
    provider::Provider,
    providers,
    usage::{
        ADD_FAVORITE_ACTION,
        REMOVE_FAVORITE_ACTION,
        UsageStore,
        remove_synthetic_duplicates,
        sort_results,
    },
};

const RECENT_RESULT_LIMIT: usize = 5;

pub struct DaemonState {
    config: Config,
    providers: Vec<Box<dyn Provider>>,
    usage: UsageStore,
}

type SharedState = Arc<Mutex<DaemonState>>;

pub struct Daemon {
    listener: UnixListener,
    state: SharedState,
}

impl Daemon {
    pub fn new(socket_path: &PathBuf) -> anyhow::Result<Self> {
        if let Err(err) = remove_stale_socket(&socket_path) {
            bail!(
                "failed to remove stale socket at {}: {err}",
                socket_path.display()
            );
        }

        let listener = match UnixListener::bind(&socket_path) {
            Ok(sock) => sock,
            Err(e) => {
                bail!("failed to bind socket at {}: {e}", socket_path.display());
            }
        };

        let config = Config::load_config().context("while reading config file")?;
        let providers = providers::build(&config);
        let usage = UsageStore::load();

        let state = Arc::new(Mutex::new(DaemonState {
            config,
            providers,
            usage,
        }));

        Ok(Self { listener, state })
    }

    pub fn run(&self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    let state = Arc::clone(&self.state);
                    thread::spawn(move || {
                        if let Err(err) = Self::handle_client(stream, state) {
                            eprintln!("failed to handle client: {err}");
                        }
                    });
                }
                Err(err) => eprintln!("failed to accept connection: {err}"),
            }
        }
    }

    fn handle_client(stream: UnixStream, state: SharedState) -> anyhow::Result<()> {
        let mut stream = BufReader::new(stream);
        let mut buf = String::new();

        loop {
            buf.clear();

            if stream.read_line(&mut buf)? == 0 {
                break Ok(());
            }

            let request: ClientRequest = serde_json::from_str(&buf)?;

            let response = {
                let mut state = state
                    .lock()
                    .expect("daemon state mutex should not be poisoned");
                state.handle_request(request)
            };

            write_response(stream.get_mut(), &response)?;
        }
    }
}

impl DaemonState {
    pub fn handle_request(&mut self, request: ClientRequest) -> ServerResponse {
        match request {
            ClientRequest::Query { request_id, query } => self.handle_query(request_id, query),
            ClientRequest::Activate {
                provider,
                id,
                action,
            } => self.handle_activate(provider, id, action),
            ClientRequest::Refresh { request_id } => self.handle_refresh(request_id),
            ClientRequest::Config => self.handle_config(),
        }
    }

    fn handle_query(&mut self, request_id: u64, query: String) -> ServerResponse {
        let mut results = vec![];

        for provider in &self.providers {
            results.extend(provider.search(&query));
        }

        self.usage.boost_results(&mut results);
        if query.trim().is_empty() {
            results.extend(self.usage.favorite_results(&results));
            results.extend(self.usage.recent_results(&results, RECENT_RESULT_LIMIT));
            remove_synthetic_duplicates(&mut results);
        }
        self.usage.add_result_actions(&mut results);
        sort_results(&mut results);

        ServerResponse::Results {
            request_id,
            items: results,
        }
    }

    fn handle_activate(&mut self, provider: String, id: String, action: String) -> ServerResponse {
        if action == ADD_FAVORITE_ACTION || action == REMOVE_FAVORITE_ACTION {
            let changed = self.usage.handle_favorite_action(&provider, &id, &action);
            if changed {
                if let Err(err) = self.usage.save() {
                    eprintln!("failed to save usage state: {err}");
                }
            }

            return ServerResponse::Activated {
                provider,
                id,
                action,
            };
        }

        let Some(provider_impl) = self.providers.iter().find(|p| p.id() == provider) else {
            return ServerResponse::Error {
                message: format!("provider not found: {provider}"),
            };
        };

        match provider_impl.activate(&id, &action) {
            Ok(()) => {
                if self.usage.record_activation(&provider, &id, &action) {
                    if let Err(err) = self.usage.save() {
                        eprintln!("failed to save usage state: {err}");
                    }
                }

                ServerResponse::Activated {
                    provider,
                    id,
                    action,
                }
            }
            Err(err) => ServerResponse::Error {
                message: err.to_string(),
            },
        }
    }

    fn handle_refresh(&mut self, request_id: u64) -> ServerResponse {
        let config = match Config::load_config() {
            Ok(config) => config,
            Err(err) => {
                return ServerResponse::Error {
                    message: format!("failed to reload config: {err}"),
                };
            }
        };

        providers::update(&mut self.providers, &config);
        self.config = config;

        let mut errors = vec![];
        for provider in &mut self.providers {
            if let Err(err) = provider.refresh() {
                errors.push(format!(
                    "provider '{}' failed to refresh: {err}",
                    provider.id()
                ));
            }
        }

        ServerResponse::Refreshed {
            request_id,
            config: self.config.clone(),
            errors,
        }
    }

    fn handle_config(&self) -> ServerResponse {
        ServerResponse::Config {
            config: self.config.clone(),
        }
    }
}

fn remove_stale_socket(socket_path: &PathBuf) -> io::Result<()> {
    match std::fs::remove_file(socket_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn write_response(stream: &mut impl Write, response: &ServerResponse) -> io::Result<()> {
    serde_json::to_writer(&mut *stream, response)?;
    stream.write_all(b"\n")?;
    stream.flush()
}
