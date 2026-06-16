use std::collections::{BTreeMap};



#[derive(Default)]
pub struct History {
    history: BTreeMap<usize, String>,
}

impl History {
    pub fn add(&mut self, cmd: &str) {
        self.history.insert(self.history.len(), cmd.to_string());
    }

    pub fn print_history(&self) -> () {
        for (k, v) in self.history.iter() {
            println!("{} {}", k + 1, v);
        }
    }

    pub fn print_history_limited(&self, limit:usize) -> () {
        for (k, v) in self.history.iter().skip(self.history.len().saturating_sub(limit)) {
            println!("{} {}", k + 1, v);
        }
    }
}