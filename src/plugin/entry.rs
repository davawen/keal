use nucleo_matcher::{Matcher, pattern::Pattern, Utf32Str};

use crate::icon::IconPath;

use super::PluginIndex;

/// Returned by the plugin manager to the UI
/// Specifies from which plugin the entry comes from
pub struct LabelledEntry<'a> {
    pub entry: Entry<'a>,
    pub plugin_index: PluginIndex
}

/// Returned by plugins to the plugin manager
pub struct Entry<'a> {
    pub name: &'a str,
    pub icon: Option<&'a IconPath>,
    pub comment: Option<&'a str>,
    /// fuzzy matching score
    pub score: u32,
    /// index in the plugin itself
    pub index: usize
}

impl<'a> Entry<'a> {
    /// creates a new entry by fuzzy matching on the name and comment
    /// returns none if nothing matches
    pub fn new(matcher: &mut Matcher, pattern: &Pattern, charbuf: &mut Vec<char>, name: &'a str, icon: Option<&'a IconPath>, comment: Option<&'a str>, index: usize) -> Option<Self> {
        let a = pattern.score(Utf32Str::new(name, charbuf), matcher);
        let b = comment.and_then(|comment| pattern.score(Utf32Str::new(comment, charbuf), matcher));
        let score = a.map(|a| b.map(|b| a + b).unwrap_or(a)).or(b)?;

        Some(Self { name, icon, comment, score, index })
    }
    
    pub fn label(self, plugin_index: PluginIndex) -> LabelledEntry<'a> {
        LabelledEntry { entry: self, plugin_index }
    }
}

