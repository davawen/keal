use std::{collections::HashMap, fs};

use fuzzy_matcher::FuzzyMatcher;
use tini::Ini;

use super::{match_span::MatchSpan, EntryTrait};

#[derive(Debug)]
pub struct DesktopEntry {
    pub name: String,
    comment: Option<String>,
    /// string that will be used for fuzzy matching
    /// concatenation of name, generic name, categories and comment
    to_match: String
}

impl DesktopEntry {
    fn new(mut ini: HashMap<String, String>) -> Self {
        let name = ini.remove("Name").unwrap();
        let comment = ini.remove("Comment");
        DesktopEntry {
            to_match: format!(
                "{name}{}{}{}",
                ini.get("GenericName").map(String::as_ref).unwrap_or(""),
                ini.get("Categories").map(String::as_ref).unwrap_or(""),
                comment.as_deref().unwrap_or(""),
            ),
            name,
            comment
        }
    }
}

impl EntryTrait for DesktopEntry {
    fn name(&self) ->  &str { &self.name }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn to_match(&self) ->  &str { &self.to_match }
}

fn xdg_directories(dir: &str) -> Vec<String> {
    let mut data_dirs = std::env::var("XDG_DATA_DIRS").map(|dirs| dirs.split(':').map(str::to_owned).collect()).unwrap_or(vec![]);
    if let Ok(home) = std::env::var("HOME") {
        data_dirs.push(format!("{home}/.local/share"));
    }

    data_dirs.into_iter().map(|path| format!("{path}/{dir}")).collect()
}

/// Returns the list of all applications on the system
pub fn desktop_entries() -> impl Iterator<Item = DesktopEntry> {
    let app_dirs = xdg_directories("applications");

    app_dirs.into_iter().flat_map(|path| {
        let entries = fs::read_dir(path)?;

        let entries = entries
            .flatten()
            .filter(|entry| entry.metadata().map(|x| !x.is_dir()).unwrap_or(true))
            .map(|entry| entry.path())
            .filter(|path| path.extension().map(|e| e == "desktop").unwrap_or(false))
            .flat_map(|path| Ini::from_file(&path))
            .map(|ini| ini.section_iter("Desktop Entry").map(|(a, b)| (a.to_owned(), b.to_owned())).collect::<HashMap<_, _>>())
            .map(DesktopEntry::new);
        
        std::io::Result::Ok(entries) // type annotations needed
    }).flatten()
}
