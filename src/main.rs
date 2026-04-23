#[allow(unused_imports)]
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;


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
        },
        Err(_) => false
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
        
        let builtins = ["echo", "exit", "type"];
        
        
        
        command = command.trim().to_string();
        
        if command == "exit" {
            break;
        }
        
        if command == "pwd" {
            println!("{}", std::env::current_dir().unwrap().display());
        }
        
        if command.starts_with("echo") {

            println!("{}", &command[5..]);
        }else if command.starts_with("type") {
            
            let command_args = &command[5..];
            
            if builtins.contains(&command_args) {
                println!("{} is a shell builtin", command_args);
            }else{
                
                match get_command_path(&path, command_args) {
                    Some(fp) => println!("{} is {}", command_args, fp.display()),
                    _ => println!("{}: not found ", command_args),
                }
            }
        }else{
            
            let mut parts = command.split_whitespace();
            let program = parts.next().unwrap();
            let args: Vec<&str> = parts.collect();
            
            
            
            match get_command_path(&path, program) {
                Some(fp) => {
                    
                    
                  let out = Command::new(fp)
                      .arg0(program)
                      .args(args)
                      .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()
                      .expect("Failed to execute command")
                      .wait()
                    .expect("Failed to wait for command");
                 
    
                },
                _ => println!("{0}: command not found", command.trim())
            }
            
            
            
        }
    }
}
