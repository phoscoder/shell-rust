use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::cell::RefCell;

use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Editor};
use rustyline::history::DefaultHistory;

use std::fs;
use std::path::{Path, PathBuf};


use std::io::{self, Write};

use crate::builtins::{BUILTINS, handle_builtins};
use crate::exec;
use crate::path;
use crate::tokenizer;

struct MyCompleter {
    last_tab: RefCell<bool>,
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
        let commands = ["echo", "exit"];
        
        let start = line[..pos].rfind(' ').map(|p| p + 1).unwrap_or(0);
        let prefix = &line[start..pos];

        let is_first_word = start == 0;
        
        let mut matches: Vec<String> = if is_first_word {
           commands
            .iter()
            .filter(|c| c.starts_with(prefix))
            .map(|c| c.to_string())
            .collect()
        } else {
          get_file_completions(prefix)  
        };  

        let mut external_matches = path::get_command_matches(std::env::var("PATH").unwrap(), prefix);
        matches.append(&mut external_matches);
        
        matches.sort();
        matches.dedup();
    
        /* ---------------- single match ---------------- */
        if matches.is_empty() {
            *self.last_tab.borrow_mut() = false;
            return Ok((start, vec![]));
        }
        
        /* ---------------- single match ---------------- */
        if matches.len() == 1 {
            // let mut s = matches[0].clone();
            // s.push(' ');
            // *self.last_tab.borrow_mut() = false;
            // return Ok((start, vec![s]));


            let mut s = matches[0].clone();
            
            if !s.ends_with('/') {
                s.push(' ');
            }
            
            return Ok((start, vec![s]));
        }
        
        /* ---------------- multiple matches ---------------- */
        let lcp = longest_common_prefix(&matches);
        
        if lcp.len() > prefix.len() {
            *self.last_tab.borrow_mut() = false;
            return Ok((start, vec![lcp]));
        }
        
        /* ---------------- TAB behavior (no LCP) ---------------- */
        let mut last_tab = self.last_tab.borrow_mut();
        
        if !*last_tab {
            print!("\x07");
            std::io::stdout().flush().unwrap();
            
            *last_tab = true; 
            
            return Ok((start, vec![]));
        }
        
        // SECOND TAB → print all matches
        println!();
        
        for m in &matches {
            print!("{} ", m);
        }
        println!();
        
        *last_tab = false;
        
        print!("$ {}", line);
        std::io::stdout().flush().unwrap();
        
        Ok((start, vec![]))
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
    
    let mut rl: Editor<MyCompleter, DefaultHistory> = Editor::new().unwrap();
    rl.set_helper(Some(MyCompleter{
        last_tab: RefCell::new(false),
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
                handle_builtins(&command, &tokens, redirect_type, &redirect_file, &path_var);

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
