use std::process::{Command, Stdio};

use std::sync::{Arc, Mutex};
use crate::completion_registry::CompletionRegistry;

use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config, Context, Helper, Editor};
use rustyline::history::DefaultHistory;

use std::fs;
use std::path::{Path, PathBuf};


use crate::builtins::{BUILTINS, handle_builtins};
use crate::exec;
use crate::jobs::JobTable;
use crate::path;
use crate::tokenizer;
use crate::history;

struct MyCompleter {
    registry: Arc<Mutex<CompletionRegistry>>,
}

impl Helper for MyCompleter {}
impl Hinter for MyCompleter {
    type Hint = String;
}

impl Highlighter for MyCompleter {}
impl Validator for MyCompleter {}

impl Completer for MyCompleter {
    type Candidate = Pair;
    
    fn complete(
        &self, 
        line: &str,
        pos: usize,
        _: &rustyline::Context
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        
        let line_to_pos = &line[..pos];
        let start = line_to_pos.rfind(' ').map(|p| p + 1).unwrap_or(0);
        let prefix = &line[start..pos];

        if let Some(mut matches) = self.get_registered_completions(line_to_pos, start, prefix) {
            matches.sort();
            matches.dedup();
            return Ok((start, to_pairs(matches)));
        }
        
        let mut matches = if start == 0 && !prefix.is_empty() && !prefix.contains('/') {
            // Completing the command position: suggest builtins + executables from $PATH.
            let mut results: Vec<String> = BUILTINS
                .iter()
                .filter(|c| c.starts_with(prefix))
                .map(|c| (*c).to_string())
                .collect();

            let path_var = std::env::var("PATH").unwrap_or_default();
            results.extend(path::get_command_matches(path_var, prefix));
            results
        } else {
            get_file_completions(prefix)
        };
        
        matches.sort();
        matches.dedup();

        Ok((start, to_pairs(matches)))
    }
}

impl MyCompleter {
    fn get_registered_completions(
        &self,
        line: &str,
        word_start: usize,
        current_word: &str,
    ) -> Option<Vec<String>> {
        let command = line.split_whitespace().next()?;

        unsafe {
            std::env::set_var("COMP_LINE", line);
            std::env::set_var("COMP_POINT", line.len().to_string());
        }

        if word_start <= command.len() {
            return None;
        }

        // The tester expects argv[3] to be the previous word before the current one.
        // For example, when the buffer is `git `, the "current word" is empty and the
        // previous word is `git`.
        let prev_word = line[..word_start].split_whitespace().last().unwrap_or("");

        let script = {
            let registry = self.registry.lock().ok()?;
            registry.get(command).cloned()
        }?;

        let output = Command::new(script)
            .arg(command)
            .arg(current_word)
            .arg(prev_word)
            .stdout(Stdio::piped())
            .output()
            .ok()?;

        let completions = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let candidate = line.trim();
                if candidate.is_empty() {
                    None
                } else {
                    Some(candidate.to_string())
                }
            })
            .collect::<Vec<_>>();

        Some(completions)
    }
}

fn to_pairs(mut matches: Vec<String>) -> Vec<Pair> {
    if matches.len() == 1 {
        let m = matches.pop().unwrap();
        let replacement = if m.ends_with('/') { m.clone() } else { format!("{m} ") };
        return vec![Pair {
            display: m,
            replacement,
        }];
    }

    matches
        .into_iter()
        .map(|m| Pair {
            display: m.clone(),
            replacement: m,
        })
        .collect()
}



fn get_file_completions(prefix: &str) -> Vec<String> {


    if prefix.is_empty() {
        return list_dir(Path::new("."), "");
    }

    let path = Path::new(prefix);

    if prefix.ends_with('/') {
        return list_dir(path, "")
    }


    let (dir, partial) = if prefix.contains('/') {
        let path = Path::new(prefix);
    
        let dir = path.parent().unwrap_or(Path::new("."));
        let partial = path.file_name()
            .unwrap_or_default()
            .to_string_lossy();
    
        (dir, partial)
    } else {
        (Path::new("."), prefix.into())
    };

    

    list_dir(dir, &partial)
}

fn list_dir(dir: &Path, partial: &str) -> Vec<String> {

    let mut results = Vec::new();
    
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if partial.is_empty() || name_str.starts_with(partial) {
                let mut full_path = if dir == Path::new(".") {
                    PathBuf::from(&*name_str)
                } else {
                    dir.join(&*name_str)
                };

                if full_path.is_dir() {
                    full_path.push("");
                }

                results.push(full_path.to_string_lossy().to_string());
            }
        }
    }

    results
}

pub fn start_shell_repl() {
    let path_var = std::env::var("PATH").unwrap();

    let mut command_hist = history::History::default();

    let registry = Arc::new(Mutex::new(CompletionRegistry::default()));
    let mut jobs = JobTable::default();
    
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .build();
    let mut rl: Editor<MyCompleter, DefaultHistory> = Editor::with_config(config).unwrap();
    rl.set_helper(Some(MyCompleter {
        registry: Arc::clone(&registry),
    }));

    

    loop {
        for line in jobs.drain_done_notifications() {
            println!("{}", line);
        }
        
        let readline = rl.readline("$ ");
        
        
        let command = match readline {
            Ok(line) => line.trim().to_string(),
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(_) => continue
        };

        // command = command.trim().to_string();
        command_hist.add(&command);
        rl.add_history_entry(&command).expect_err("Could not save command");

        let (tokens, (redirect_type, redirect_file)) = tokenizer::tokenize(&command);
        // NOTE: keep the repl output clean for Codecrafters tests.
        
        if tokens.is_empty() {
            continue;
        }
        

        if tokens.iter().any(|t| t == "|") {
            exec::run_pipeline(
                &tokens,
                &path_var,
                redirect_type,
                redirect_file,
                &registry,
                &mut jobs
            );
            continue;
        }

        if BUILTINS.contains(&tokens[0].as_str()) {
            let should_break =
                handle_builtins(
                    &command, 
                    &tokens, 
                    redirect_type, 
                    &redirect_file, 
                    &path_var,
                    &registry, 
                    &mut jobs,
                    &mut command_hist,
                );

            if should_break {
                break;
            }
        } else {
            if tokens.is_empty() {
                continue;
            }

            if let Some(child) = exec::run_external(&tokens, &path_var, redirect_type, redirect_file) {
                let command_line = tokens.join(" ");
                let (job_id, pid) = jobs.add(child, command_line);
                println!("[{}] {}", job_id, pid);
            }
        }
    }
}
