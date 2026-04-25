use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub fn get_command_path(path: &str, command: &str) -> Option<PathBuf> {
    path.split(":")
        .map(Path::new)
        .filter(|p| p.is_dir())
        .map(|p| p.join(command))
        .find(|fp| fp.is_file() && is_executable(&fp))
}


pub fn get_command_matches(path: String, prefix: &str) -> Vec<String> {
    let mut results = Vec::new();
    
    for dir in path.split(":") {
        let dir_path = Path::new(dir);
        
        if !dir_path.is_dir() {
            continue;
        }
        
        if let Ok(entries) = fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with(prefix) && is_executable(&path) {
                        results.push(name.to_string() + " ");
                    }
                }
            }
        }
    }
    
    results
}

pub fn is_executable(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            let mode = metadata.permissions().mode();
            mode & 0o111 != 0
        }
        Err(_) => false,
    }
}