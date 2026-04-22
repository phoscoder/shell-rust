#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();
        
        
        if command.trim().to_string() == "exit" {
            break;
        }
        
        if command.starts_with("echo") {
            println!("{}", &command[5..].trim());
        }else{
            println!("{0}: command not found", command.trim());
        }
    }
}
