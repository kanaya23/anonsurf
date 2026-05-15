use std::{
    env, fs,
    path::{Path, PathBuf},
};

const FALLBACK_PATHS: [&str; 6] = [
    "/usr/local/sbin",
    "/usr/local/bin",
    "/usr/sbin",
    "/usr/bin",
    "/sbin",
    "/bin",
];

pub fn command_exists(command: &str) -> bool {
    let path = Path::new(command);
    if path.components().count() > 1 {
        return is_executable(path);
    }

    let mut search_dirs: Vec<PathBuf> = env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
        .collect();

    for fallback in FALLBACK_PATHS {
        let fallback = PathBuf::from(fallback);
        if !search_dirs.iter().any(|dir| dir == &fallback) {
            search_dirs.push(fallback);
        }
    }

    search_dirs
        .iter()
        .map(|dir| dir.join(command))
        .any(|candidate| is_executable(&candidate))
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::command_exists;
    use std::{fs, os::unix::fs::PermissionsExt};

    #[test]
    fn explicit_executable_path_is_detected() {
        let temp = std::env::temp_dir().join(format!("anonsurf-core-cmd-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        let script = temp.join("testcmd");
        fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();

        let script_path = script.to_string_lossy();
        assert!(command_exists(script_path.as_ref()));

        let _ = fs::remove_dir_all(&temp);
    }
}
