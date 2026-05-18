use std::{
    process::Command as StdCommand,
    thread,
};

use anyhow::{
    Context,
    bail,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::provider::{
    Provider,
    SearchResult,
};

pub struct CommandsProvider {
    commands: Vec<Command>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CommandsProviderConfig {
    pub enabled: bool,
    pub cmds: Vec<Command>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    name: String,
    command: String,
}

impl Default for CommandsProviderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cmds: vec![],
        }
    }
}

impl CommandsProvider {
    pub fn new(config: &CommandsProviderConfig) -> Self {
        Self {
            commands: config.cmds.clone(),
        }
    }
}

impl Provider for CommandsProvider {
    fn id(&self) -> &'static str {
        "commands"
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        let mut results = vec![];
        let query = query.to_lowercase();

        for cmd in &self.commands {
            let name = cmd.name.to_lowercase();
            if name.contains(&query) {
                results.push(SearchResult {
                    id: cmd.name.clone(),
                    provider: self.id(),
                    title: cmd.name.to_string(),
                    subtitle: String::new(),
                    icon: "builtin:terminal".to_string(),
                    score: 1.0,
                    actions: vec!["run".to_string()],
                    autocomplete: None,
                });
            }
        }

        results
    }

    fn activate(&self, id: &str, action: &str) -> anyhow::Result<()> {
        match action {
            "run" => {
                let Some(command) = self.commands.iter().find(|cmd| cmd.name == id) else {
                    bail!("command not found: {id}");
                };

                let mut child = StdCommand::new("sh")
                    .arg("-c")
                    .arg(&command.command)
                    .spawn()
                    .context("while attempting to spawn command")?;

                thread::spawn(move || {
                    if let Err(err) = child.wait() {
                        eprintln!("failed to reap command: {err}");
                    }
                });

                Ok(())
            }
            _ => bail!("unsupported command action: {action}"),
        }
    }

    fn refresh(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
