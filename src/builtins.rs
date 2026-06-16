use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::Path;

use std::sync::{Arc, Mutex};

use crate::completion_registry::CompletionRegistry;

use crate::path;

pub const BUILTINS: [&str; 8] = [
    "echo", "exit", "type", "pwd", "cd", "complete", "jobs", "history",
];

pub fn is_builtin(cmd: &str) -> bool {
    BUILTINS.contains(&cmd)
}

// Minimal builtin execution used by pipelines.
// This avoids `println!` so we can write into a pipe or redirected output.
//
// Returns `true` if the shell should exit (only for `exit`).
pub fn run_builtin_piped(
    tokens: &[String],
    path: &str,
    registry: &Arc<Mutex<CompletionRegistry>>,
    jobs: &mut crate::jobs::JobTable,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> bool {
    let _ = stdin; // Most builtins here don't read stdin yet.
    let _ = stderr; // Keep signature for future parity.

    if tokens.is_empty() {
        return false;
    }

    let cmd = tokens[0].as_str();

    if cmd == "echo" {
        let output = if tokens.len() > 1 {
            tokens[1..].join(" ")
        } else {
            String::new()
        };
        let _ = writeln!(stdout, "{}", output);
        return false;
    }

    if cmd == "type" {
        if tokens.len() < 2 {
            return false;
        }
        let arg = tokens[1].as_str();
        if BUILTINS.contains(&arg) {
            let _ = writeln!(stdout, "{} is a shell builtin", arg);
        } else {
            match path::get_command_path(path, arg) {
                Some(fp) => {
                    let _ = writeln!(stdout, "{} is {}", arg, fp.display());
                }
                None => {
                    // Match existing output (note trailing space).
                    let _ = writeln!(stdout, "{}: not found ", arg);
                }
            }
        }
        return false;
    }

    if cmd == "pwd" {
        let _ = writeln!(stdout, "{}", std::env::current_dir().unwrap().display());
        return false;
    }

    if cmd == "jobs" {
        // Reuse the existing jobs printing behavior for now (writes to terminal).
        // Not typically used in pipeline tests.
        jobs.get_jobs();
        return false;
    }

    if cmd == "complete" {
        // Support `complete -p <cmd>` in pipelines.
        if tokens.len() >= 3 && tokens[1] == "-p" {
            let reg = registry.lock().unwrap();
            if let Some(script) = reg.get(&tokens[2]) {
                let _ = writeln!(stdout, "complete -C '{}' {}", script.display(), tokens[2]);
            } else {
                let _ = writeln!(
                    stdout,
                    "complete: {}: no completion specification",
                    tokens[2]
                );
            }
        }
        return false;
    }

    if cmd == "exit" {
        return true;
    }

    // `cd` in a pipeline is unusual; treat as a no-op here.
    false
}

pub fn handle_builtins(
    command: &str,
    tokens: &[String],
    redirect_type: i8,
    redirect_file: &Option<String>,
    path: &str,
    registry: &Arc<Mutex<CompletionRegistry>>,
    jobs: &mut crate::jobs::JobTable,
    command_hist: &mut crate::history::History<'_>,
) -> bool {
    if command.starts_with("echo") {
        let output = if tokens.len() > 1 {
            tokens[1..].join(" ")
        } else {
            String::new()
        };

        match redirect_type {
            1 => {
                if let Some(file) = &redirect_file {
                    match std::fs::File::create(file) {
                        Ok(mut f) => {
                            writeln!(f, "{}", output).unwrap();
                        }
                        Err(e) => {
                            writeln!(io::stderr(), "{}", e).unwrap();
                        }
                    }
                } else {
                    println!("{}", output);
                }
            }
            2 => {
                println!("{}", output);

                if let Some(file) = &redirect_file {
                    let _ = std::fs::File::create(file);
                }
            }
            3 => {
                if let Some(file) = &redirect_file {
                    match OpenOptions::new().create(true).append(true).open(file) {
                        Ok(mut f) => {
                            writeln!(f, "{}", output).unwrap();
                        }
                        Err(e) => {
                            writeln!(io::stderr(), "{}", e).unwrap();
                        }
                    }
                } else {
                    println!("{}", output);
                }
            }
            4 => {
                println!("{}", output);

                if let Some(file) = &redirect_file {
                    let _ = std::fs::File::create(file);
                }
            }

            _ => {}
        }
    } else if command.starts_with("jobs") {
        jobs.get_jobs();
    } else if command.starts_with("complete") {
        if command.contains("-p") {
            let reg = registry.lock().unwrap();
            if let Some(script) = reg.get(&tokens[2]) {
                println!("complete -C '{}' {}", script.display(), tokens[2]);
            } else {
                println!("complete: {}: no completion specification", tokens[2]);
            }
        }

        if command.contains("-r") {
            let mut reg = registry.lock().unwrap();
            reg.remove(&tokens[2]);
        }

        if command.contains("-C") {
            let mut reg = registry.lock().unwrap();
            reg.register(&tokens[3], std::path::PathBuf::from(&tokens[2]));
        }
    } else if command.starts_with("history") {
        let has_limit = command.contains(" ");
        if has_limit {
            let limit = command.split(" ").nth(1).unwrap().parse::<usize>().unwrap();
            command_hist.print_history_limited(limit);
        } else {

            if command.contains("-r") {
                let path = command.split(" ").nth(1).unwrap();
                command_hist.load_history(path).unwrap();
            }
            command_hist.print_history();
        }
    } else if command.starts_with("pwd") {
        println!("{}", std::env::current_dir().unwrap().display());
    } else if command.starts_with("cd") {
        let home_path = std::env::var("HOME").unwrap();
        let cd_args = &command[3..].replace("~", &home_path);
        let dir_path = Path::new(cd_args);

        if dir_path.is_dir() {
            std::env::set_current_dir(&dir_path).unwrap();
        } else {
            println!("cd: {}: No such file or directory", cd_args)
        }
    } else if command.starts_with("type") {
        let command_args = &command[5..];

        if BUILTINS.contains(&command_args) {
            println!("{} is a shell builtin", command_args);
        } else {
            match path::get_command_path(&path, command_args) {
                Some(fp) => println!("{} is {}", command_args, fp.display()),
                _ => println!("{}: not found ", command_args),
            }
        }
    } else if command == "exit" {
        return true;
    }

    false
}
