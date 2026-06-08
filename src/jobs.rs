use std::collections::BTreeMap;
use std::process::{Child, ExitStatus};

#[derive(Debug)]
#[allow(dead_code)]
pub struct Job {
    pub id: u32,
    pub pid: u32,
    pub command: String,
    child: Child,
}

#[derive(Default)]
pub struct JobTable {
    next_id: u32,
    pub jobs: BTreeMap<u32, Job>,
}

impl JobTable {
    pub fn add(&mut self, child: Child, command: String) -> (u32, u32) {
        self.next_id += 1;
        let id = self.next_id;
        let pid = child.id();
        self.jobs.insert(
            id,
            Job {
                id,
                pid,
                command,
                child,
            },
        );
        (id, pid)
    }

    pub fn reap_finished(&mut self) -> Vec<(u32, u32, ExitStatus)> {
        let mut done = Vec::new();
        let ids: Vec<u32> = self.jobs.keys().copied().collect();
        for id in ids {
            let finished = {
                let job = self.jobs.get_mut(&id).expect("job disappeared");
                match job.child.try_wait() {
                    Ok(Some(status)) => Some((job.pid, status)),
                    Ok(None) => None,
                    Err(_) => None,
                }
            };
            if let Some((pid, status)) = finished {
                self.jobs.remove(&id);
                done.push((id, pid, status));
            }
        }
        done
    }

    fn sign_for(&self, id: u32) -> &'static str {
        let Some(max) = self.jobs.keys().next_back().copied() else {
            return " ";
        };
        if id == max {
            return "+";
        }
        let prev = self.jobs.keys().nth_back(1).copied();
        if prev.is_some_and(|p| p == id) {
            "-"
        } else {
            " "
        }
    }

    fn format_line(id: u32, sign: &str, status: &str, command: &str) -> String {
        // Bash-ish layout. Codecrafters MA9 expects status padded so that:
        // "[1]+  Done                 cat /tmp/..." matches.
        format!("[{}]{}  {:<21}{}", id, sign, status, command)
    }

    // Called before showing a new prompt: emit completion notifications for
    // finished jobs and remove them from the table.
    pub fn drain_done_notifications(&mut self) -> Vec<String> {
        let ids: Vec<u32> = self.jobs.keys().copied().collect();
        let mut out = Vec::new();

        for id in ids {
            let finished = {
                let Some(job) = self.jobs.get_mut(&id) else { continue; };
                job.child.try_wait().ok().flatten().is_some()
            };
            if finished {
                let sign = self.sign_for(id);
                if let Some(job) = self.jobs.remove(&id) {
                    let cmd = job.command.trim_end_matches('&').trim_end();
                    out.push(Self::format_line(id, sign, "Done", cmd));
                }
            }
        }

        out
    }

    pub fn get_jobs(&mut self) -> () {
        let ids: Vec<u32> = self.jobs.keys().copied().collect();
        let mut done_ids: Vec<u32> = Vec::new();

        for id in ids {
            let sign = self.sign_for(id);
            let (status, cmd, finished) = {
                let job = self.jobs.get_mut(&id).unwrap();
                let finished = job.child.try_wait().ok().flatten().is_some();
                let status = if finished { "Done" } else { "Running" };
                let cmd = job.command.trim_end_matches('&').trim_end().to_string();
                (status, cmd, finished)
            };

            if !cmd.trim().is_empty() {
                println!("{}", Self::format_line(id, sign, status, &cmd));
            }

            if finished {
                done_ids.push(id);
            }
        }

        for id in done_ids {
            self.jobs.remove(&id);
        }
    }
}
