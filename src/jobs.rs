use std::io::{stdout, Write};
use std::process::Child;
use std::sync::Mutex;

pub static JOB_TABLE: Mutex<JobTable> = Mutex::new(JobTable::new());

pub struct Job {
    pub id: usize,
    pub pid: u32,
    pub command: String,
    pub child: Option<Child>,
}

pub struct JobTable {
    jobs: Vec<Job>,
    next_id: usize,
}

impl JobTable {
    pub const fn new() -> Self {
        Self {
            jobs: Vec::new(),
            next_id: 1,
        }
    }

    /// Register a background child process
    pub fn add(&mut self, command: String, child: Child) -> (usize, u32) {
        let id = self.next_id;
        self.next_id += 1;
        let pid = child.id();

        self.jobs.push(Job {
            id,
            pid,
            command,
            child: Some(child),
        });

        (id, pid)
    }

    /// Check all running jobs non-blockingly and collect completed jobs
    pub fn poll_completed(&mut self) -> Vec<(usize, String, Option<i32>)> {
        let mut completed = Vec::new();
        let mut indices_to_remove = Vec::new();

        for (idx, job) in self.jobs.iter_mut().enumerate() {
            if let Some(ref mut child) = job.child {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        completed.push((job.id, job.command.clone(), status.code()));
                        indices_to_remove.push(idx);
                    }
                    Ok(None) => {}
                    Err(_) => {
                        indices_to_remove.push(idx);
                    }
                }
            }
        }

        for idx in indices_to_remove.into_iter().rev() {
            self.jobs.remove(idx);
        }

        completed
    }

    /// List active background jobs (`jobs` builtin)
    pub fn list(&mut self) {
        let completed = self.poll_completed();
        for (id, cmd, code) in completed {
            println!("[{}]+ Done ({})                 {}", id, code.unwrap_or(0), cmd);
        }

        if self.jobs.is_empty() {
            return;
        }

        let total = self.jobs.len();
        for (i, job) in self.jobs.iter().enumerate() {
            let symbol = if i + 1 == total {
                "+"
            } else if i + 2 == total {
                "-"
            } else {
                " "
            };
            println!(
                "[{}] {} Running                 {} (pid: {})",
                job.id, symbol, job.command, job.pid
            );
        }
    }

    /// Bring a background job to the foreground (`fg [%id]`)
    pub fn bring_to_foreground(&mut self, target_id: Option<usize>) -> Result<(), String> {
        let idx = match target_id {
            Some(id) => self
                .jobs
                .iter()
                .position(|j| j.id == id)
                .ok_or_else(|| format!("bnb: fg: job %{} not found", id))?,
            None => {
                if self.jobs.is_empty() {
                    return Err("bnb: fg: no current jobs".to_string());
                }
                self.jobs.len() - 1
            }
        };

        let mut job = self.jobs.remove(idx);
        println!("{}", job.command);
        let _ = stdout().flush();

        if let Some(mut child) = job.child.take() {
            let status = child
                .wait()
                .map_err(|e| format!("bnb: fg: failed to wait on child process: {}", e))?;

            if !status.success() {
                if let Some(code) = status.code() {
                    return Err(format!("process exited with code {}", code));
                }
            }
        }

        Ok(())
    }
}

/// Print notifications for completed background jobs right before displaying the prompt
pub fn print_completed_jobs() {
    if let Ok(mut table) = JOB_TABLE.lock() {
        let finished = table.poll_completed();
        for (id, cmd, code) in finished {
            let status_desc = match code {
                Some(0) | None => "\x1b[1;32mDone\x1b[0m".to_string(),
                Some(c) => format!("\x1b[1;31mExit {}\x1b[0m", c),
            };
            println!("[{}]+ {}                 {}", id, status_desc, cmd);
        }
    }
}