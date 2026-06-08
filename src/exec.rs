
use std::process::{Child, Command, Stdio};
use std::os::unix::process::CommandExt;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::builtins;
use crate::completion_registry::CompletionRegistry;
use crate::jobs::JobTable;
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

fn open_redirect_writers(
    redirect_type: i8,
    redirect_file: Option<String>,
) -> (Box<dyn Write + Send>, Box<dyn Write + Send>) {
    let mut out: Box<dyn Write + Send> = Box::new(io::stdout());
    let mut err: Box<dyn Write + Send> = Box::new(io::stderr());

    match redirect_type {
        1 => {
            if let Some(file) = redirect_file {
                if let Ok(f) = std::fs::File::create(file) {
                    out = Box::new(f);
                }
            }
        }
        2 => {
            if let Some(file) = redirect_file {
                if let Ok(f) = std::fs::File::create(file) {
                    err = Box::new(f);
                }
            }
        }
        3 => {
            if let Some(file) = redirect_file {
                if let Ok(f) = std::fs::OpenOptions::new().append(true).create(true).open(file) {
                    out = Box::new(f);
                }
            }
        }
        4 => {
            if let Some(file) = redirect_file {
                if let Ok(f) = std::fs::OpenOptions::new().append(true).create(true).open(file) {
                    err = Box::new(f);
                }
            }
        }
        _ => {}
    }

    (out, err)
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
    registry: &Arc<Mutex<CompletionRegistry>>,
    jobs: &mut JobTable,
) {
    // Split the pipeline into command segments.
    let mut cmds: Vec<Vec<String>> = Vec::new();
    let mut cur: Vec<String> = Vec::new();
    for t in tokens {
        if t == "|" {
            if !cur.is_empty() {
                cmds.push(cur);
                cur = Vec::new();
            }
        } else {
            cur.push(t.clone());
        }
    }
    if !cur.is_empty() {
        cmds.push(cur);
    }

    if cmds.len() < 2 {
        run_external(&tokens.to_vec(), path_var, redirect_type, redirect_file);
        return;
    }

    // For now, keep the more complex builtin-aware implementation only for 2-stage pipelines
    // (needed for previous stage tests). Multi-stage pipelines in this stage are external-only.
    if cmds.len() > 2 {
        run_pipeline_external_only(&cmds, path_var, redirect_type, redirect_file);
        return;
    }

    let left = cmds[0].as_slice();
    let right = cmds[1].as_slice();

    let left_cmd = left[0].as_str();
    let right_cmd = right[0].as_str();
    let left_is_builtin = builtins::is_builtin(left_cmd);
    let right_is_builtin = builtins::is_builtin(right_cmd);

    // Helper for the final output (right side) when it is a builtin.
    let (mut builtin_stdout, mut builtin_stderr) = open_redirect_writers(redirect_type, redirect_file.clone());

    // builtin | builtin (buffer in memory; enough for this stage).
    if left_is_builtin && right_is_builtin {
        let mut buf: Vec<u8> = Vec::new();
        let mut stdin = io::empty();
        let mut stderr = io::sink();
        let _ = builtins::run_builtin_piped(left, path_var, registry, jobs, &mut stdin, &mut buf, &mut stderr);

        let mut cursor = io::Cursor::new(buf);
        let mut stderr2 = io::stderr();
        let _ = builtins::run_builtin_piped(
            right,
            path_var,
            registry,
            jobs,
            &mut cursor,
            builtin_stdout.as_mut(),
            &mut stderr2,
        );
        return;
    }

    // builtin | external
    if left_is_builtin && !right_is_builtin {
        let right_prog = right_cmd;
        let right_args: Vec<&str> = right[1..].iter().map(|s| s.as_str()).collect();
        let Some(right_fp) = path::get_command_path(path_var, right_prog) else {
            println!("{0}: command not found", right_prog.trim());
            return;
        };

        let (right_stdout, right_stderr) = apply_redirect(redirect_type, redirect_file);

        let mut right_child = match Command::new(right_fp)
            .arg0(right_prog)
            .args(right_args)
            .stdin(Stdio::piped())
            .stdout(right_stdout)
            .stderr(right_stderr)
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut right_stdin = match right_child.stdin.take() {
            Some(s) => s,
            None => return,
        };

        let mut stdin = io::empty();
        let mut stderr = io::stderr();
        let _ = builtins::run_builtin_piped(left, path_var, registry, jobs, &mut stdin, &mut right_stdin, &mut stderr);
        drop(right_stdin); // EOF to the right side
        let _ = right_child.wait();
        return;
    }

    // external | builtin
    if !left_is_builtin && right_is_builtin {
        let left_prog = left_cmd;
        let left_args: Vec<&str> = left[1..].iter().map(|s| s.as_str()).collect();
        let Some(left_fp) = path::get_command_path(path_var, left_prog) else {
            println!("{0}: command not found", left_prog.trim());
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

        let mut left_stdout = match left_child.stdout.take() {
            Some(s) => s,
            None => return,
        };

        // Drain the producer output so it can't block if the builtin doesn't read stdin.
        let drain_handle = thread::spawn(move || {
            let mut sink = io::sink();
            let _ = io::copy(&mut left_stdout, &mut sink);
        });

        let mut stdin = io::empty();
        let _ = builtins::run_builtin_piped(
            right,
            path_var,
            registry,
            jobs,
            &mut stdin,
            builtin_stdout.as_mut(),
            builtin_stderr.as_mut(),
        );

        let _ = left_child.kill();
        let _ = left_child.wait();
        let _ = drain_handle.join();
        return;
    }

    // external | external
    let left_prog = left_cmd;
    let left_args: Vec<&str> = left[1..].iter().map(|s| s.as_str()).collect();
    let right_prog = right_cmd;
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

    let _ = right_child.wait();
    let _ = left_child.kill();
    let _ = left_child.wait();
}

fn run_pipeline_external_only(
    cmds: &[Vec<String>],
    path_var: &str,
    redirect_type: i8,
    redirect_file: Option<String>,
) {
    let last_idx = cmds.len() - 1;
    let mut children: Vec<std::process::Child> = Vec::new();
    let mut prev_out: Option<std::process::ChildStdout> = None;

    for (i, cmd_tokens) in cmds.iter().enumerate() {
        if cmd_tokens.is_empty() {
            return;
        }
        let prog = &cmd_tokens[0];
        let args: Vec<&str> = cmd_tokens[1..].iter().map(|s| s.as_str()).collect();

        let Some(fp) = path::get_command_path(path_var, prog) else {
            println!("{0}: command not found", prog.trim());
            return;
        };

        let stdin = match prev_out.take() {
            Some(p) => Stdio::from(p),
            None => Stdio::inherit(),
        };

        let (stdout, stderr) = if i == last_idx {
            apply_redirect(redirect_type, redirect_file.clone())
        } else {
            (Stdio::piped(), Stdio::inherit())
        };

        let mut child = match Command::new(fp)
            .arg0(prog)
            .args(args)
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return,
        };

        if i != last_idx {
            prev_out = child.stdout.take();
        }
        children.push(child);
    }

    // Wait for the last (consumer) first. Then ensure producers are terminated.
    if let Some(mut last) = children.pop() {
        let _ = last.wait();
    }
    for mut child in children.into_iter().rev() {
        let _ = child.kill();
        let _ = child.wait();
    }
}
