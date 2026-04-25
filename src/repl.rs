use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::cell::RefCell;

use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Editor};
use rustyline::history::DefaultHistory;


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
        
        let mut matches: Vec<String> = commands
            .iter()
            .filter(|c| c.starts_with(prefix))
            .map(|c| c.to_string())
            .collect();

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
            let mut s = matches[0].clone();
            s.push(' ');
            *self.last_tab.borrow_mut() = false;
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

pub fn start_shell_repl() {
    let path_var = std::env::var("PATH").unwrap();
    
    let mut rl: Editor<MyCompleter, DefaultHistory> = Editor::new().unwrap();
    rl.set_helper(Some(MyCompleter{
        last_tab: RefCell::new(false),
    }));

    loop {
        // print!("$ ");
        // io::stdout().flush().unwrap();

        // let mut command = String::new();
        // io::stdin().read_line(&mut command).unwrap();
        
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
