
use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;
use crate::path;

pub fn run_external(
    tokens: &Vec<String>, 
    path_var: &str, 
    redirect_type: i8,
    redirect_file: Option<String>
) {
    
    
    let program = &tokens[0];
    let args: Vec<&str> = tokens[1..].iter().map(|s| s.as_str()).collect();

    match path::get_command_path(&path_var, program) {
        Some(fp) => {
            let mut stdout = Stdio::inherit();
            let mut stderr = Stdio::inherit();

            match redirect_type {
                1 => {
                    if let Some(file) = redirect_file {
                        let f = std::fs::File::create(file).expect("failed to open file");
                        stdout = Stdio::from(f);
                    }
                }
                2 => {
                    if let Some(file) = redirect_file {
                        let f = std::fs::File::create(file).expect("failed to open file");
                        stderr = Stdio::from(f);
                    }
                }
                _ => {}
            }

            Command::new(fp)
                .arg0(program)
                .args(args)
                .stdin(Stdio::inherit())
                .stdout(stdout)
                .stderr(stderr)
                .spawn()
                .expect("Failed to execute command")
                .wait()
                .expect("Failed to wait for command");
        }
        _ => println!("{0}: command not found", program.trim()),
    }
    
    
}