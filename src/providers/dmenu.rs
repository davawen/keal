use std::{iter::Peekable, io::Lines};

use crate::{entries::EntryTrait, icon::IconPath, arguments::Protocol};

use super::plugin::execution::read_entry_from_stream;

pub struct DmenuEntry {
    pub name: String,
    icon: Option<IconPath>,
    comment: Option<String>
}

impl EntryTrait for DmenuEntry {
    fn name(&self) -> &str { &self.name }
    fn icon(&self) -> Option<&IconPath> { self.icon.as_ref() }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn to_match(&self) -> &str { &self.name }
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

pub fn read_dmenu_entries(protocol: Protocol) -> Vec<DmenuEntry> {
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
    entries
}
