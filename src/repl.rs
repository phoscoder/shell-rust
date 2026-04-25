use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

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

struct MyCompleter; 

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
        
        let matches: Vec<String> = commands
            .iter()
            .filter(|c| c.starts_with(prefix))
            .map(|c| {
                let mut s = c.to_string();
                s.push(' ');
                s
            })
            .collect();
        Ok((start, matches))
    }
}

pub fn start_shell_repl() {
    let path_var = std::env::var("PATH").unwrap();
    
    let mut rl: Editor<MyCompleter, DefaultHistory> = Editor::new().unwrap();
    rl.set_helper(Some(MyCompleter));

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

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
