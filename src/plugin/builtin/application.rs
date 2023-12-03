use std::{path::Path, process};

use nucleo_matcher::{Matcher, pattern::Pattern, Utf32Str, Utf32String};
use walkdir::WalkDir;

use crate::{icon::{IconPath, Icon}, ini_parser::Ini, plugin::{Plugin, PluginExecution, Entry, Action}, xdg_utils::xdg_directories, config::Config};

#[derive(Debug)]
struct DesktopEntry {
    name: String,
    comment: Option<String>,
    icon: Option<IconPath>,
    /// other strings that will be used for fuzzy matching
    /// concatenation of generic name, categories, and keywords
    /// this won't be used for display purpose, so it's directory converted to a nucleo `Utf32String`
    to_match: Utf32String,
    exec: String,
    path: Option<String>,
    terminal: bool
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
        ).into();
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

pub struct ApplicationPlugin(Vec<DesktopEntry>);

impl ApplicationPlugin {
    /// Creates a `Plugin` with an `ApplicationPlugin` generator
    /// `current_desktop` is the `$XDG_CURRENT_DESKTOP` environment variable
    pub fn create(current_desktop: String) -> Plugin {
        Plugin {
            name: "Applications".to_owned(),
            prefix: "app".to_owned(),
            icon: None,
            comment: Some("Launch applications on the system".to_owned()),
            generator: Box::new(move |_, _| {
                let current_desktop: Vec<&str> = current_desktop.split(':').collect();
                let app_dirs = xdg_directories("applications");

                // for every `.../share/application` directory
                let entries = app_dirs.into_iter().flat_map(|path| {
                    // get every subdirectory
                    let entries = WalkDir::new(path)
                        .follow_links(true)
                        .into_iter();

                    // get every .desktop file, and parse them
                    let entries = entries
                        .flatten()
                        .filter(|entry| entry.metadata().map(|x| !x.is_dir()).unwrap_or(true))
                        .map(|entry| entry.into_path())
                        .filter(|path| path.extension().map(|e| e == "desktop").unwrap_or(false))
                        .flat_map(|path| Some((Ini::from_file(&path, &['#']).ok()?, path)))
                        .flat_map(|(ini, path)| DesktopEntry::new(ini, &path, &current_desktop));
                    entries
                });

                Box::new(ApplicationPlugin(entries.collect()))
            })
        }
    }
}

impl PluginExecution for ApplicationPlugin {
    fn finished(&mut self) -> bool { false }
    fn wait(&mut self) {}
    fn send_query(&mut self, _: &Config, _: &str) -> Action { Action::None }

    fn send_enter(&mut self, config: &Config, _: &str, idx: Option<usize>) -> Action {
        let Some(idx) = idx else { return Action::None };
        let app = &self.0[idx];

        let mut command = if app.terminal {
            let mut command = process::Command::new(&config.terminal_path);
            command.arg("-e");
            command.arg("sh");
            command
        } else {
            process::Command::new("sh")
        };
        command.arg("-c").arg(&app.exec);
        if let Some(path) = &app.path {
            command.current_dir(path);
        }
        Action::Exec(command)
    }

    fn get_entries<'a>(&'a self, _: &Config, matcher: &mut Matcher, pattern: &Pattern) -> Vec<Entry<'a>> {
        let mut charbuf = vec![];

        self.0.iter().enumerate().flat_map(|(index, entry)| {
            let a = pattern.score(Utf32Str::new(&entry.name, &mut charbuf), matcher);
            let b = entry.comment.as_ref().and_then(|c| pattern.score(Utf32Str::new(c, &mut charbuf), matcher));
            let c = pattern.score(entry.to_match.slice(..), matcher);

            let score = a.map(|a| b.map(|b| a + b).unwrap_or(a)).or(b)
                .map(|a_b| c.map(|c| a_b + c).unwrap_or(a_b)).or(c)?;

            Some(Entry {
                name: &entry.name,
                icon: entry.icon.as_ref(),
                comment: entry.comment.as_deref(),
                score, index
            })
        }).collect()
    }

    fn get_name(&self, index: usize) -> &str {
        &self.0[index].name
    }
}
