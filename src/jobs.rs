use std::collections::BTreeMap;
use std::process::{Child};

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

    // Reap finished background children to avoid zombies.
    // Kept silent to avoid interfering with Codecrafters stage output.
    // pub fn reap_finished(&mut self) -> Vec<(u32, u32, ExitStatus)> {
    //     let mut done = Vec::new();
    //     let ids: Vec<u32> = self.jobs.keys().copied().collect();
    //     for id in ids {
    //         let finished = {
    //             let job = self.jobs.get_mut(&id).expect("job disappeared");
    //             match job.child.try_wait() {
    //                 Ok(Some(status)) => Some((job.pid, status)),
    //                 Ok(None) => None,
    //                 Err(_) => None,
    //             }
    //         };
    //         if let Some((pid, status)) = finished {
    //             self.jobs.remove(&id);
    //             done.push((id, pid, status));
    //         }
    //     }
    //     done
    // }

    // Implements `jobs`: prints known background jobs and their state.
    // This will also detect completed jobs via `try_wait()` and then drop them.
    pub fn get_jobs(&mut self) -> () {
        // Collect keys first so we don't keep an immutable borrow of `self.jobs`
        // while calling `is_finished()` (which needs a mutable borrow).
        let job_keys: Vec<u32> = self.jobs.keys().copied().collect();
        let job_count = job_keys.len() as u32;

        for key in job_keys {
            let sign = if job_count.saturating_sub(key) == 0 {
                "+"
            } else if job_count.saturating_sub(key) == 1 {
                "-"
            } else {
                " "
            };

            let state = if self.is_finished(key) { "Done" } else { "Running" };

            if let Some(job) = self.jobs.get(&key) {
                if !job.command.trim().is_empty() {
                    let cleaned_command = job.command.trim_end_matches('&').trim_end();
                    println!("[{}]{}  {} {}", key, sign, state, cleaned_command);
                }
            }
        }
    }

    fn is_finished(&mut self, id: u32) -> bool {
        let Some(job) = self.jobs.get_mut(&id) else {
            return true;
        };
        job.child.try_wait().ok().flatten().is_some()
    }
}
