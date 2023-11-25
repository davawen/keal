use std::path::Path;

use walkdir::WalkDir;

use crate::{entries::EntryTrait, icon::{IconPath, Icon}, xdg_utils::xdg_directories, ini_parser::Ini};

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
    /// `location` is the path to the desktop file
    /// `current_desktop` is the `$XDG_CURRENT_DESKTOP` environment variable, split by colon
    fn new(mut ini: Ini, location: &Path, current_desktop: &[&str]) -> Option<Self> {
        let mut ini = ini
            .remove_section("Desktop Entry")?
            .into_map();

        if ini.get("Type")? != "Application" {
            return None
        }
        
        if let Some(no_display) = ini.get("NoDisplay") {
            if no_display == "true" { return None }
        }

        // TODO: handle `Hidden` key: https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#recognized-keys

        if let Some(only_show_in) = ini.get("OnlyShowIn") {
            let contained = only_show_in.split(';').filter(|s| !s.is_empty()).any(|x| current_desktop.contains(&x));
            if !contained { return None }
        }

        if let Some(not_show_in) = ini.get("NotShowIn") {
            let contained = not_show_in.split(';').filter(|s| !s.is_empty()).any(|x| current_desktop.contains(&x));
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
        let exec = parse_exec_key(ini.remove("Exec")?, &name, location, icon.as_ref());
        let path = ini.remove("Path");
        let terminal = ini.get("Terminal").map(|v| v == "true").unwrap_or(false);

        Some(DesktopEntry {
            name, comment, icon, to_match,
            exec, path, terminal
        })
    }
}

/// https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html#exec-variables
/// `name`, `location` and `icon` are required for the `%c`, `%k` and `%i` codes
fn parse_exec_key(exec: String, name: &str, location: &Path, icon: Option<&IconPath>) -> String {
    // unsure how it could be possible to avoid reallocating...
    // since modifying the string in place might entail large moves that would be worse
    // in the end, most of those strings will be less 128 bytes, so I guess it doesn't really matter in the end.
    let mut out = String::with_capacity(exec.capacity());
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        match c {
            '%' => if let Some(c) = chars.next() {
                match c {
                    '%' => out.push('%'),
                    'f' | 'F' | 'u' | 'U' => (), // don't expand "input parameters"
                    'd' | 'D' | 'n' | 'N' | 'v' | 'm' => (), // deprecated codes
                    'i' => match icon { // insert `--icon {icon name}`
                        Some(IconPath::Name(name)) if !name.is_empty() => out.push_str(&format!("--icon {name}")),
                        Some(IconPath::Path(Icon::Svg(path) | Icon::Other(path))) if !path.as_os_str().is_empty() => if let Some(path) = path.to_str() {
                            out.push_str(&format!("--icon {path}"))
                        }
                        _ => ()
                    }
                    'c' => out.push_str(name), // supposed to be the translated name.  TODO: handle locales
                    'k' => if let Some(location) = location.to_str() {
                        out.push_str(location)
                    }
                    _ => () // malformed code
                }
            },
            c => out.push(c)
        }
    }

    out
}

impl EntryTrait for DesktopEntry {
    fn name(&self) ->  &str { &self.name }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn icon(&self) -> Option<&IconPath> { self.icon.as_ref() }
    fn to_match(&self) ->  &str { &self.to_match }
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
            .flat_map(|path| Some((Ini::from_file(&path, &['#']).ok()?, path)))
            .flat_map(|(ini, path)| DesktopEntry::new(ini, &path, current_desktop));
        
        std::io::Result::Ok(entries) // type annotations needed
    }).flatten()
}
