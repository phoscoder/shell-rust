#[allow(unused_imports)]
use std::io::{self, Write};
use std::path::Path;
use std::os::unix::fs::PermissionsExt;
use std::fmt::Display;


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
        
        if command.starts_with("echo") {

            println!("{}", &command[5..]);
        }else if command.starts_with("type") {
            
            let command_args = &command[5..];
            
            if builtins.contains(&command_args) {
                println!("{} is a shell builtin", command_args);
            }else{
                // println!("{}: not found", command_args);
                
                let matched_path = path.split(":")
                    .map(Path::new)
                    .filter(|p| p.is_dir())
                    .map(|p| p.join(command_args))
                    .find(|fp| fp.is_file() && is_executable(&fp.display().to_string()));
                
                match matched_path {
                    Some(fp) => println!("{} is {:?}", command_args, fp ),
                    _ => println!("{}: not found ", command_args),
                }
            }
        }else{
            println!("{0}: command not found", command.trim());
        }
    }
}
