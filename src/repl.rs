use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

use std::io::{self, Write};

use crate::builtins::{BUILTINS, handle_builtins};
use crate::exec;
use crate::path;
use crate::tokenizer;

pub fn start_shell_repl() {
    let path_var = std::env::var("PATH").unwrap();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();

        command = command.trim().to_string();

        let (tokens, (redirect_type, redirect_file)) = tokenizer::tokenize(&command);

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
