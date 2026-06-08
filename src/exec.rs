
use std::process::{Child, Command, Stdio};
use std::os::unix::process::CommandExt;
use crate::path;

fn apply_redirect(redirect_type: i8, redirect_file: Option<String>) -> (Stdio, Stdio) {
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

    (stdout, stderr)
}

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
            let (stdout, stderr) = apply_redirect(redirect_type, redirect_file);

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

pub fn run_pipeline(
    tokens: &[String],
    path_var: &str,
    redirect_type: i8,
    redirect_file: Option<String>,
) {
    // Only support one `|` (two commands) for this stage.
    let Some(pipe_idx) = tokens.iter().position(|t| t == "|") else {
        run_external(&tokens.to_vec(), path_var, redirect_type, redirect_file);
        return;
    };

    let (left, right_with_bar) = tokens.split_at(pipe_idx);
    let right = &right_with_bar[1..];
    if left.is_empty() || right.is_empty() {
        return;
    }

    let left_prog = &left[0];
    let left_args: Vec<&str> = left[1..].iter().map(|s| s.as_str()).collect();
    let right_prog = &right[0];
    let right_args: Vec<&str> = right[1..].iter().map(|s| s.as_str()).collect();

    let Some(left_fp) = path::get_command_path(path_var, left_prog) else {
        println!("{0}: command not found", left_prog.trim());
        return;
    };
    let Some(right_fp) = path::get_command_path(path_var, right_prog) else {
        println!("{0}: command not found", right_prog.trim());
        return;
    };

    let mut left_child = match Command::new(left_fp)
        .arg0(left_prog)
        .args(left_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let left_stdout = match left_child.stdout.take() {
        Some(s) => s,
        None => return,
    };

    // Apply any parsed redirection to the *right* side of the pipeline.
    let (right_stdout, right_stderr) = apply_redirect(redirect_type, redirect_file);

    let mut right_child = match Command::new(right_fp)
        .arg0(right_prog)
        .args(right_args)
        .stdin(Stdio::from(left_stdout))
        .stdout(right_stdout)
        .stderr(right_stderr)
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    // Wait for the consumer first. After it exits, the producer should usually
    // terminate due to SIGPIPE; we also try to kill it to avoid hanging.
    let _ = right_child.wait();
    let _ = left_child.kill();
    let _ = left_child.wait();
}
