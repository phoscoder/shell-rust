use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Default)]
pub struct CompletionRegistry {
    by_command: BTreeMap<String, PathBuf>,
}

impl CompletionRegistry {
    pub fn register(&mut self, cmd: &str, script: PathBuf) {
        self.by_command.insert(cmd.to_string(), script);
    }

    pub fn get(&self, cmd: &str) -> Option<&PathBuf> {
        self.by_command.get(cmd)
    }

    pub fn remove(&mut self, cmd: &str) {
        self.by_command.remove(cmd);
    }
}