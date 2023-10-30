use std::collections::HashMap;

use super::EntryTrait;

pub struct Plugin {
    prefix: String,
    comment: Option<String>,
    exec: String
}

impl Plugin {
    pub fn generate(&self) -> impl Iterator<Item = FieldEntry> {
        match self.prefix.as_str() {
            "sm" => vec![FieldEntry { field: "power off".to_owned(), comment: None, icon: None }],
            "em" => vec![FieldEntry { field: "nerd".to_owned(), comment: None, icon: None }],
            _ => todo!()
        }.into_iter()
    }
}

pub struct Plugins(HashMap<String, Plugin>);

pub fn get_plugins() -> Plugins {
    Plugins(HashMap::from([
        ("sm".to_owned(), Plugin { prefix: "sm".to_owned(), exec: "/home/davawen/.config/keal/plugins/session_manager/exec".to_owned(), comment: None }),
        ("em".to_owned(), Plugin { prefix: "em".to_owned(), exec: "/home/davawen/.config/keal/plugins/emoji/exec".to_owned(), comment: Some("Get emojis from their name".to_owned()) })
    ]))
}

impl Plugins {
    /// If filter starts with a plugin name and a space, returns the given plugin and the remainder of the string to fuzzy match.
    pub fn filter_starts_with_plugin<'a, 'b>(&'a self, filter: &'b str) -> Option<(&'a Plugin, &'b str)> {
        let (name, remainder) = filter.split_once(' ')?;
        let plugin = self.0.get(name)?;

        Some((plugin, remainder))
    }
}

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
