
use std::process::{Child, Command, Stdio};
use std::os::unix::process::CommandExt;
use crate::path;

pub fn run_external(
    tokens: &Vec<String>, 
    path_var: &str, 
    redirect_type: i8,
    redirect_file: Option<String>
) -> Option<Child> {

    let mut mut_tokens = tokens.clone();

    let is_background = mut_tokens.last().map_or(false, |t| t == "&");

    if is_background {
        mut_tokens.pop();
    }
    
    if mut_tokens.is_empty() {
        return None;
    }
    
    let program = &mut_tokens[0];
    let args: Vec<&str> = mut_tokens[1..].iter().map(|s| s.as_str()).collect();

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
                3 => {
                    if let Some(file) = redirect_file {
                        let append_file = std::fs::OpenOptions::new()
                                        .append(true)
                                        .create(true)
                                        .open(file)
                                        .expect("failed to open file");
                        stdout = Stdio::from(append_file); 
                            
                    }
                }
                4 => {
                    if let Some(file) = redirect_file {
                        let append_file = std::fs::OpenOptions::new()
                                        .append(true)
                                        .create(true)
                                        .open(file)
                                        .expect("failed to open file");
                        stderr = Stdio::from(append_file);  
                            
                    }
                }
                _ => {}
            }

            let mut cmd = Command::new(fp);
            cmd
                .arg0(program)
                .args(args)
                // A background job should not read from the terminal by default.
                .stdin(if is_background { Stdio::null() } else { Stdio::inherit() })
                .stdout(stdout)
                .stderr(stderr);

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(_) => return None,
            };

            if is_background {
                return Some(child);
            }

            let _ = child.wait();
            None
        }
        _ => {
            println!("{0}: command not found", program.trim());
            None
        }
    }
    
    
}
