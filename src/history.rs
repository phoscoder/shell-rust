use crate::repl::MyCompleter;
use rustyline::history::History as RlHistory;
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
            println!("{:>5}  {}", k + 1, v);
        }
    }

    pub fn print_history_limited(&self, limit: usize) {
        let total = self.rl.history().len();
        let start = total.saturating_sub(limit);
        for (k, v) in self.rl.history().iter().skip(start).enumerate() {
            println!("{:>5}  {}", start + k + 1, v);
        }
    }

    pub fn load_history(&mut self, path: &str) -> Result<(), rustyline::error::ReadlineError> {
        self.rl.load_history(path)
    }
}
