use std::{collections::HashMap, io::BufRead};

use super::EntryTrait;

#[derive(Clone)]
pub struct Plugin {
    pub prefix: String,
    pub comment: Option<String>,
    pub exec: String
}

impl Plugin {
    pub fn generate(&self) -> impl Iterator<Item = FieldEntry> {
        use std::process::{Stdio, Command};

        let mut plug = Command::new(&self.exec)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn().expect("Couldn't spawn process from plugin");

        let stdout = plug.stdout.take().unwrap();
        let stdout = std::io::BufReader::new(stdout);

        let mut v = vec![];
        let mut current = None;
        for line in stdout.lines().flatten() {
            match line.get(0..2) {
                Some("F:") => {
                    if let Some(old) = current {
                        v.push(old);
                    }
                    current = Some(FieldEntry {
                        field: line[2..].to_owned(),
                        icon: None,
                        comment: None
                    });
                }
                Some("I:") => if let Some(ref mut current) = current {
                    current.icon = Some(line[2..].to_owned())
                }
                Some("C:") => if let Some(ref mut current) = current {
                    current.comment = Some(line[2..].to_owned())
                }
                Some("E:") => if let Some(current) = current {
                    v.push(current);
                    break
                }
                Some(d) => panic!("unknown descriptor `{d}` in plugin {}", self.prefix),
                None => panic!("no descriptor given in plugin {}", self.prefix)
            }
        }

        v.into_iter()
    }
}

pub struct Plugins(HashMap<String, Plugin>);

impl Plugins {
    /// If filter starts with a plugin name and a space, returns the given plugin and the remainder of the string to fuzzy match.
    pub fn filter_starts_with_plugin<'a, 'b>(&'a self, filter: &'b str) -> Option<(&'a Plugin, &'b str)> {
        let (name, remainder) = filter.split_once(' ')?;
        let plugin = self.0.get(name)?;

        Some((plugin, remainder))
    }

    pub fn get(&self, prefix: &str) -> Option<&Plugin> {
        self.0.get(prefix)
    }
}

pub fn get_plugins() -> Plugins {
    Plugins(HashMap::from([
        ("sm".to_owned(), Plugin { prefix: "sm".to_owned(), exec: "/home/davawen/.config/keal/plugins/session_manager/exec.sh".to_owned(), comment: None }),
        ("em".to_owned(), Plugin { prefix: "em".to_owned(), exec: "/home/davawen/.config/keal/plugins/emoji/exec".to_owned(), comment: Some("Get emojis from their name".to_owned()) })
    ]))
}

#[derive(Debug)]
pub struct PluginEntry {
    prefix: String,
    comment: Option<String>
}

impl EntryTrait for PluginEntry {
    fn name(&self) ->  &str { &self.prefix }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn to_match(&self) ->  &str { &self.prefix }
}

pub fn plugin_entries(plugins: &Plugins) -> impl Iterator<Item = PluginEntry> + '_ {
    plugins.0.values()
        .map(|plugin| PluginEntry { prefix: plugin.prefix.clone(), comment: plugin.comment.clone() })
}

#[derive(Debug)]
pub struct FieldEntry {
    field: String,
    icon: Option<String>,
    comment: Option<String>
}

impl EntryTrait for FieldEntry {
    fn name(&self) ->  &str { &self.field }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn to_match(&self) ->  &str { &self.field }
}
