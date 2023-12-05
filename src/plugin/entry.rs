use nucleo_matcher::{Matcher, pattern::Pattern, Utf32Str};

use crate::icon::IconPath;

use super::PluginIndex;

/// Returned by plugins to the plugin manager
#[derive(Debug)]
pub struct Entry<'a> {
    pub name: &'a str,
    pub icon: Option<&'a IconPath>,
    pub comment: Option<&'a str>,
    /// fuzzy matching score
    pub score: u32,
    pub label: Label
}

#[derive(Debug)]
pub struct OwnedEntry {
    pub name: String,
    pub icon: Option<IconPath>,
    pub comment: Option<String>,
    /// fuzzy matching score
    pub score: u32,
    pub label: Label
}

/// Specifies the origin of the entry
#[derive(Debug, Clone, Copy)]
pub struct Label {
    /// plugin it comes from
    pub plugin_index: PluginIndex,
    /// index in the plugin itself
    pub index: usize
}

impl Label {
    pub fn index(index: usize) -> Self {
        Self { plugin_index: PluginIndex::default(), index }
    }

    fn with_plugin(self, plugin_index: PluginIndex) -> Self {
        Self { plugin_index, index: self.index }
    }
}

impl<'a> Entry<'a> {
    /// creates a new entry by fuzzy matching on the name and comment
    /// returns none if nothing matches
    pub fn new(matcher: &mut Matcher, pattern: &Pattern, charbuf: &mut Vec<char>, name: &'a str, icon: Option<&'a IconPath>, comment: Option<&'a str>, index: usize) -> Option<Self> {
        let a = pattern.score(Utf32Str::new(name, charbuf), matcher);
        let b = comment.and_then(|comment| pattern.score(Utf32Str::new(comment, charbuf), matcher));
        let score = a.map(|a| b.map(|b| a + b).unwrap_or(a)).or(b)?;

        Some(Self { name, icon, comment, score, label: Label::index(index) })
    }

    pub fn label(self, plugin_index: PluginIndex) -> Self {
        Self {
            label: self.label.with_plugin(plugin_index),
            ..self
        }
    }
    
    pub fn to_owned(&self) -> OwnedEntry {
        OwnedEntry {
            name: self.name.to_owned(),
            icon: self.icon.cloned(),
            comment: self.comment.map(str::to_owned),
            score: self.score,
            label: self.label
        }
    }
}

