use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub fn get_command_path(path: &str, command: &str) -> Option<PathBuf> {
    path.split(":")
        .map(Path::new)
        .filter(|p| p.is_dir())
        .map(|p| p.join(command))
        .find(|fp| fp.is_file() && is_executable(&fp.display().to_string()))
}

pub fn is_executable(path: &str) -> bool {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            let mode = metadata.permissions().mode();
            mode & 0o111 != 0
        }
        Err(_) => false,
    }
}