use std::{collections::HashMap, path::{PathBuf, Path}};

use tini::Ini;
use walkdir::WalkDir;

use crate::icon::IconPath;

use super::EntryTrait;

#[derive(Debug)]
pub struct DesktopEntry {
    name: String,
    comment: Option<String>,
    pub icon: Option<IconPath>,
    /// cache the string that will be used for fuzzy matching
    /// concatenation of name, generic name, categories, keywords and comment
    to_match: String,
    pub exec: String,
    pub path: Option<String>,
    pub terminal: bool
}

impl DesktopEntry {
    /// `ini` is the .desktop file as parsed by `tini`.
    /// `current_desktop` is the `$XDG_CURRENT_DESKTOP` environment variable, split by colon
    fn new(ini: Ini, current_desktop: &[&str]) -> Option<Self> {
        let mut ini: HashMap<_, _> = ini
            .section_iter("Desktop Entry")
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect();

        if ini.get("Type")? != "Application" {
            return None
        }
        
        if let Some(no_display) = ini.get("NoDisplay") {
            if no_display == "true" { return None }
        }

        // TODO: handle `Hidden` key: https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#recognized-keys

        if let Some(only_show_in) = ini.get("OnlyShowIn") {
            let contained = only_show_in.split(';').any(|x| current_desktop.contains(&x));
            if !contained { return None }
        }

        if let Some(not_show_in) = ini.get("NotShowIn") {
            let contained = not_show_in.split(';').any(|x| current_desktop.contains(&x));
            if contained { return None }
        }

        let name = ini.remove("Name")?;
        let comment = ini.remove("Comment");
        let icon = ini.remove("Icon").map(|i| IconPath::new(i, None));
        let to_match = format!("{name}{}{}{}{}",
            ini.get("GenericName").map(String::as_ref).unwrap_or(""),
            ini.get("Categories").map(String::as_ref).unwrap_or(""),
            ini.get("Keywords").map(String::as_ref).unwrap_or(""),
            comment.as_deref().unwrap_or(""),
        );
        let exec = ini.remove("Exec")?;
        let path = ini.remove("Path");
        let terminal = ini.get("Terminal").map(|v| v == "true").unwrap_or(false);

        Some(DesktopEntry {
            name, comment, icon, to_match,
            exec, path, terminal
        })
    }
}

impl EntryTrait for DesktopEntry {
    fn name(&self) ->  &str { &self.name }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn icon(&self) -> Option<&IconPath> { self.icon.as_ref() }
    fn to_match(&self) ->  &str { &self.to_match }
}

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

/// Returns the list of all applications on the system
/// `current_desktop` is the `$XDG_CURRENT_DESKTOP` environment variable split by colon
pub fn desktop_entries<'a>(current_desktop: &'a [&str]) -> impl Iterator<Item = DesktopEntry> + 'a {
    let app_dirs = xdg_directories("applications");

    app_dirs.into_iter().flat_map(|path| {
        let entries = WalkDir::new(path)
            .follow_links(true)
            .into_iter();

        let entries = entries
            .flatten()
            .filter(|entry| entry.metadata().map(|x| !x.is_dir()).unwrap_or(true))
            .map(|entry| entry.into_path())
            .filter(|path| path.extension().map(|e| e == "desktop").unwrap_or(false))
            .flat_map(|path| Ini::from_file(&path))
            .flat_map(|ini| DesktopEntry::new(ini, current_desktop));
        
        std::io::Result::Ok(entries) // type annotations needed
    }).flatten()
}
