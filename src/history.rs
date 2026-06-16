use crate::repl::MyCompleter;
use rustyline::{Editor, error::ReadlineError, history::DefaultHistory};

pub struct History<'a> {
    rl: &'a mut Editor<MyCompleter, DefaultHistory>,
}

impl<'a> History<'a> {
    pub fn new(rl: &'a mut Editor<MyCompleter, DefaultHistory>) -> Self {
        Self { rl }
    }

    pub fn readline(&mut self, prompt: &str) -> Result<String, ReadlineError> {
        self.rl.readline(prompt)
    }

    pub fn add(&mut self, cmd: &str) {
        let _ = self.rl.add_history_entry(cmd);
    }

    pub fn print_history(&self) {
        for (k, v) in self.rl.history().iter().enumerate() {
            println!("{} {}", k + 1, v);
        }
    }

    pub fn print_history_limited(&self, limit: usize) {
        let target: Vec<_> = self.rl.history().iter().rev().take(limit).collect();
        for (k, v) in target.iter().rev().enumerate() {
            println!("{} {}", k + 1, v);
        }
    }
}
