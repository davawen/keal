use std::path::{Path, PathBuf};

pub fn xdg_directories<P: AsRef<Path>>(dir: P) -> Vec<PathBuf> {
    let mut data_dirs: Vec<_> = std::env::var("XDG_DATA_DIRS")
        .map(|dirs| dirs.split(':').map(PathBuf::from).collect())
        .unwrap_or_default();

    if let Ok(home) = std::env::var("XDG_DATA_HOME") {
        data_dirs.push(home.into());
    }

    for path in &mut data_dirs {
        path.push(&dir);
    }

    data_dirs
}

/// Returns the path equivalent to `~/.config/keal`
pub fn config_dir() -> Result<PathBuf, &'static str> {
    let mut dir = if let Some(config) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(config)
    } else if let Some(home) = std::env::var_os("HOME") {
        Path::new(&home).join(".config")
    } else {
        return Err("neither $XDG_CONFIG_HOME nor $HOME are enabled. Didn't load any plugin.");
    };
    dir.push("keal");

    Ok(dir)
}

