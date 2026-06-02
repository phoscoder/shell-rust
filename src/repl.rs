use std::process::{Command, Stdio};

use std::sync::{Arc, Mutex};
use crate::completion_registry::CompletionRegistry;

use rustyline::completion::Completer;
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
use crate::path;
use crate::tokenizer;

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
    type Candidate = String;
    
    fn complete(
        &self, 
        line: &str,
        pos: usize,
        _: &rustyline::Context
    ) -> rustyline::Result<(usize, Vec<String>)> {
        
        let line_to_pos = &line[..pos];
        let start = line_to_pos.rfind(' ').map(|p| p + 1).unwrap_or(0);
        let prefix = &line[start..pos];

        if let Some(completions) = self.get_registered_completions(line_to_pos, start) {
            return Ok((start, completions));
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
    
        /* ---------------- single match ---------------- */
        if matches.is_empty() {
            return Ok((start, vec![]));
        }
        
        /* ---------------- single match ---------------- */
        if matches.len() == 1 {
            let mut s = matches[0].clone();
            
            if !s.ends_with('/') {
                s.push(' ');
            }
            
            return Ok((start, vec![s]));
        }
        
        /* ---------------- multiple matches ---------------- */
        let lcp = longest_common_prefix(&matches);
        
        if lcp.len() > prefix.len() {
            return Ok((start, vec![lcp]));
        }

        Ok((start, matches))
    }
}

impl MyCompleter {
    fn get_registered_completions(&self, line: &str, word_start: usize) -> Option<Vec<String>> {
        let command = line.split_whitespace().next()?;

        if word_start <= command.len() {
            return None;
        }

        let script = {
            let registry = self.registry.lock().ok()?;
            registry.get(command).cloned()
        }?;

        let output = Command::new(script).stdout(Stdio::piped()).output().ok()?;

        let completions = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let candidate = line.trim_end();
                if candidate.is_empty() {
                    None
                } else {
                    Some(format!("{} ", candidate))
                }
            })
            .collect::<Vec<_>>();

        Some(completions)
    }
}



fn longest_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    
    let mut prefix = strings[0].clone();
    
    for s in strings.iter().skip(1) {
        let mut new_prefix = String::new();
        
        for (a, b) in prefix.chars().zip(s.chars()) {
            if a == b {
                new_prefix.push(a);
            } else {
                break;
            }
        }
        
        prefix = new_prefix;
        
        if prefix.is_empty() {
            break;
        }
     }
     
     prefix
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

    let registry = Arc::new(Mutex::new(CompletionRegistry::default()));
    
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .build();
    let mut rl: Editor<MyCompleter, DefaultHistory> = Editor::with_config(config).unwrap();
    rl.set_helper(Some(MyCompleter {
        registry: Arc::clone(&registry),
    }));

    loop {
        
        let readline = rl.readline("$ ");
        
        
        let command = match readline {
            Ok(line) => line.trim().to_string(),
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(_) => continue
        };

        // command = command.trim().to_string();

        let (tokens, (redirect_type, redirect_file)) = tokenizer::tokenize(&command);
        
        // println!("redirect type: {}", redirect_type);
        
        if tokens.is_empty() {
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
                    &registry);

            if should_break {
                break;
            }
        } else {
            if tokens.is_empty() {
                continue;
            }

            exec::run_external(&tokens, &path_var, redirect_type, redirect_file);
        }
    }
}
