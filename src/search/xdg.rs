use std::collections::HashMap;

use tini::Ini;
use walkdir::WalkDir;

use super::EntryTrait;

#[derive(Debug)]
pub struct DesktopEntry {
    name: String,
    comment: Option<String>,
    /// cache the string that will be used for fuzzy matching
    /// concatenation of name, generic name, categories and comment
    to_match: String,
    pub exec: String,
    pub icon: Option<String>
}

impl DesktopEntry {
    fn new(ini: Ini) -> Option<Self> {
        let mut ini: HashMap<_, _> = ini
            .section_iter("Desktop Entry")
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect();

        let name = ini.remove("Name")?;
        let comment = ini.remove("Comment");
        let to_match = format!("{name}{}{}{}",
            ini.get("GenericName").map(String::as_ref).unwrap_or(""),
            ini.get("Categories").map(String::as_ref).unwrap_or(""),
            comment.as_deref().unwrap_or(""),
        );
        let exec = ini.remove("Exec")?;
        let icon = ini.remove("Icon");

        Some(DesktopEntry {
            name, comment, to_match,
            exec, icon
        })
    }
}

impl EntryTrait for DesktopEntry {
    fn name(&self) ->  &str { &self.name }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn icon(&self) -> Option<&str> { self.icon.as_deref() }
    fn to_match(&self) ->  &str { &self.to_match }
}

pub fn xdg_directories(dir: &str) -> Vec<String> {
    let mut data_dirs: Vec<_> = std::env::var("XDG_DATA_DIRS")
        .map(|dirs| dirs.split(':').map(str::to_owned).collect())
        .unwrap_or_default();

    if let Ok(home) = std::env::var("HOME") {
        data_dirs.push(format!("{home}/.local/share"));
    }

    data_dirs.into_iter().map(|path| format!("{path}/{dir}")).collect()
}

/// Returns the list of all applications on the system
/// Uses `collisions` to avoid putting the same application twice in the list
pub fn desktop_entries() -> impl Iterator<Item = DesktopEntry> {
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
            .flat_map(DesktopEntry::new);
        
        std::io::Result::Ok(entries) // type annotations needed
    }).flatten()
}
