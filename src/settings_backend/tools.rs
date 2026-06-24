//! Optional integration discovery. This module deliberately has no GTK types.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Returns whether `name` resolves to an executable file on the current PATH.
pub fn program_exists(name: &str) -> bool {
    if name.contains('/') {
        return executable(Path::new(name));
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|directory| directory.join(name))
        .any(|path| executable(&path))
}

fn executable(path: &Path) -> bool {
    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_is_discovered_from_path() {
        assert!(program_exists("sh"));
    }

    #[test]
    fn nonsense_tool_is_missing() {
        assert!(!program_exists("niri-shell-definitely-not-a-program"));
    }
}
