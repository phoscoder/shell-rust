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
    jobs: BTreeMap<u32, Job>,
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
}
