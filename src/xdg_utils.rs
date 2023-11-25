use std::path::{Path, PathBuf};

pub fn xdg_directories<P: AsRef<Path>>(dir: P) -> Vec<PathBuf> {
    let mut data_dirs: Vec<_> = std::env::var("XDG_DATA_DIRS")
        .unwrap_or("/usr/local/share:/usr/share".to_owned()) .split(':').map(PathBuf::from).collect();

    if let Some(home) = std::env::var_os("XDG_DATA_HOME") {
        data_dirs.push(home.into());
    } else if let Some(home) = std::env::var_os("HOME") {
        data_dirs.push(Path::new(&home).join(".local/share"))
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
        return Err("neither $XDG_CONFIG_HOME nor $HOME are defined");
    };
    dir.push("keal");

    Ok(dir)
}

/// Returns the path equivalent to `~/.local/state/keal`
pub fn state_dir() -> Result<PathBuf, &'static str> {
    let mut dir = if let Some(state) = std::env::var_os("XDG_STATE_HOME") {
        PathBuf::from(state)
    } else if let Some(home) = std::env::var_os("HOME") {
        Path::new(&home).join(".local/state")
    } else {
        return Err("neither $XDG_STATE_HOME nor $HOME are defined");
    };
    dir.push("keal");

    Ok(dir)
}
