#[allow(unused_imports)]
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if escape_next {
            
            if ch == '\\' {
                current_token += "\\";
            } else {
                current_token.push(ch);
            }
            escape_next = false;
            continue;
        }
        
        match ch {
            '\\' if !in_single_quote && !in_double_quote => {
                escape_next = true;
            }
            '\'' if !in_double_quote => {
                // Toggle single quote mode
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                // Whitespace outside quotes: end current token
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            }
            _ => {
                // Regular character or whitespace inside quotes
                current_token.push(ch);
            }
        }
    }

    // Push final token if any
    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    tokens
}

fn get_command_path(path: &str, command: &str) -> Option<PathBuf> {
    path.split(":")
        .map(Path::new)
        .filter(|p| p.is_dir())
        .map(|p| p.join(command))
        .find(|fp| fp.is_file() && is_executable(&fp.display().to_string()))
}

fn is_executable(path: &str) -> bool {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            let mode = metadata.permissions().mode();
            mode & 0o111 != 0
        }
        Err(_) => false,
    }
}

fn main() {
    // TODO: Uncomment the code below to pass the first stage

    let path = std::env::var("PATH").unwrap();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();

        let builtins = ["echo", "exit", "type", "pwd", "cd"];

        command = command.trim().to_string();

        if command == "exit" {
            break;
        }

        if command.starts_with("echo") {
            // println!("{}", &command[5..]);
            let tokens = tokenize(&command);
                if tokens.len() > 1 {
                    println!("{}", tokens[1..].join(" "));
                } else {
                    println!();
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

            if builtins.contains(&command_args) {
                println!("{} is a shell builtin", command_args);
            } else {
                match get_command_path(&path, command_args) {
                    Some(fp) => println!("{} is {}", command_args, fp.display()),
                    _ => println!("{}: not found ", command_args),
                }
            }
        } else {
            let mut parts = command.split_whitespace();
            // let program = parts.next().unwrap();
            // let args: Vec<&str> = parts.collect();

            let tokens = tokenize(&command);

            if tokens.is_empty() {
                continue;
            }

            let program = &tokens[0];
            let args: Vec<&str> = tokens[1..].iter().map(|s| s.as_str()).collect();

            match get_command_path(&path, program) {
                Some(fp) => {
                    Command::new(fp)
                        .arg0(program)
                        .args(args)
                        .stdin(Stdio::inherit())
                        .stdout(Stdio::inherit())
                        .stderr(Stdio::inherit())
                        .spawn()
                        .expect("Failed to execute command")
                        .wait()
                        .expect("Failed to wait for command");
                }
                _ => println!("{0}: command not found", command.trim()),
            }
        }
    }
}
