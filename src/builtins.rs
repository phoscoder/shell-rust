
use std::io::{self, Write};
use std::path::{Path};
use std::fs::OpenOptions;

use std::sync::{Arc, Mutex};
use crate::completion_registry::CompletionRegistry;

use crate::path;

pub const BUILTINS: [&str; 7] = ["echo", "exit", "type", "pwd", "cd", "complete", "jobs"];

pub fn handle_builtins(
    command: &str,
    tokens: &[String],
    redirect_type: i8,
    redirect_file: &Option<String>,
    path: &str,
    registry: &Arc<Mutex<CompletionRegistry>>
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
                    match OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(file)
                    {
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
        println!("");
    } else if command.starts_with("complete") {
        if command.contains("-p") {
        
            let reg = registry.lock().unwrap();
            if let Some(script) = reg.get(&tokens[2]) {
                println!("complete -C '{}' {}", script.display(), tokens[2]);
            }else {
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
