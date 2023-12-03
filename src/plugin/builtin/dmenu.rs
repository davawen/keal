use std::{iter::Peekable, io::Lines};
use crate::{icon::IconPath, arguments::Protocol, plugin::{Plugin, PluginExecution, Action, Entry}, config::Config};
use super::user::read_entry_from_stream;

struct DmenuEntry {
    pub name: String,
    icon: Option<IconPath>,
    comment: Option<String>
}

impl DmenuEntry {
    /// Creates a new dmenu entry from a line using rofi's extended dmenu protocol
    /// To set an icon for an entry, append \0icon\x1f<icon-name> to the name of the entry
    fn new_from_rofi_extended(line: &str) -> Option<Self> {
        if let Some((name, icon)) = line.split_once('\0') {
            let Some(("icon", icon)) = icon.split_once('\x1f') else { return None };

            Some(Self {
                name: name.to_owned(),
                icon: Some(IconPath::new(icon.to_owned(), None)),
                comment: None
            })
        } else {
            Some(Self {
                name: line.to_owned(),
                icon: None,
                comment: None
            })
        }
    }

    fn new_from_keal(lines: &mut Peekable<Lines<std::io::StdinLock>>) -> Self {
        let (name, icon, comment) = read_entry_from_stream(lines, None);
        Self { name, icon, comment }
    }
}

pub struct DmenuPlugin(Vec<DmenuEntry>);

impl DmenuPlugin {
    /// creates a `Plugin` with a `DmenuPlugin` generator
    pub fn create(protocol: Protocol) -> Plugin {
        Plugin {
            name: "Dmenu".to_owned(),
            prefix: "\0".to_owned(), // using an untypable null character, since this plugin's prefix should never be used
            icon: None,
            comment: None,
            generator: Box::new(move |_, _| {
                // reads entries from stdin
                let mut entries = vec![];
                let mut stdin = std::io::stdin().lines().peekable();
                while stdin.peek().is_some() {
                    let entry = match protocol {
                        Protocol::RofiExtended => {
                            let Ok(line) = stdin.next().unwrap() else { break };
                            let Some(entry) = DmenuEntry::new_from_rofi_extended(&line) else { continue };
                            entry
                        }
                        Protocol::Keal => DmenuEntry::new_from_keal(&mut stdin)
                    };
                    entries.push(entry);
                }

                Box::new(DmenuPlugin(entries))
            })
        }
    }
}

impl PluginExecution for DmenuPlugin {
    fn finished(&mut self) -> bool { false }
    fn wait(&mut self) { }
    fn send_query(&mut self, _: &crate::config::Config, _: &str) -> Action { Action::None }

    fn send_enter(&mut self, _: &crate::config::Config, query: &str, idx: Option<usize>) -> Action {
        if let Some(idx) = idx {
            let entry = &self.0[idx];
            Action::PrintAndClose(entry.name.clone())
        } else { // no choice
            Action::PrintAndClose(query.to_owned())
        }
    }

    fn get_entries<'a>(&'a self, _: &Config, matcher: &mut nucleo_matcher::Matcher, pattern: &nucleo_matcher::pattern::Pattern) -> Vec<Entry<'a>> {
        let mut charbuf = vec![];
        self.0.iter().enumerate().flat_map(|(index, entry)| {
            Entry::new(matcher, pattern, &mut charbuf, &entry.name, entry.icon.as_ref(), entry.comment.as_deref(), index)
        }).collect()
    }

    fn get_name(&self, index: usize) -> &str {
        &self.0[index].name
    }
}
