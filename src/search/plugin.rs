use super::EntryTrait;

pub struct PluginEntry {
    prefix: String,
    exec: String,
    comment: Option<String>
}

impl EntryTrait for PluginEntry {
    fn name(&self) ->  &str { &self.prefix }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn to_match(&self) ->  &str { &self.prefix }
}

pub fn plugin_entries() -> impl Iterator<Item = PluginEntry> {
    let plugins = [
        PluginEntry { prefix: "sm".to_owned(), exec: "/home/davawen/.config/keal/plugins/session_manager/exec".to_owned(), comment: None },
        PluginEntry { prefix: "em".to_owned(), exec: "/home/davawen/.config/keal/plugins/emoji/exec".to_owned(), comment: Some("Get emojis from their name".to_owned()) }
    ];

    plugins.into_iter()
}
