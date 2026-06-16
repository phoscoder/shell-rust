

#[derive(Default)]
pub struct History {
    history: Vec<String>,
}

impl History {
    pub fn add(&mut self, cmd: &str) {
        self.history.push(cmd.to_string());
    }

    pub fn print_history(&self) -> () {
        for (index, hist) in self.history.iter().enumerate() {
            println!("{} {}", index + 1, hist);
        }
    }

    pub fn print_history_limited(&self, limit:usize) -> () {
        let last_n = self.history[self.history.len().saturating_sub(limit)..].to_vec();
        for (index, hist) in last_n.iter().enumerate() {
            println!("{} {}", index + 1, hist);
        }
    }
}