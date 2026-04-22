#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();
        
        
        command = command.trim().to_string();
        
        if command == "exit" {
            break;
        }
        
        if command.starts_with("echo") {
            println!("{}", &command[5..]);
        }else{
            println!("{0}: command not found", command.trim());
        }
    }
}
