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
        let score = a.map(|a| b.map(|b| a + b).unwrap_or(2*a)).or(b)?;

        Some(Self { name, icon, comment, score, label: Label::index(index) })
    }

    pub fn label(self, plugin_index: PluginIndex) -> Self {
        Self {
            label: self.label.with_plugin(plugin_index),
            ..self
        }
    }
    
    pub fn to_display(&self, pattern: &Pattern, matcher: &mut Matcher, charbuf: &mut Vec<char>) -> DisplayEntry {
        DisplayEntry {
            name: HighlightedString::build(self.name.to_owned(), pattern, matcher, charbuf),
            icon: self.icon.cloned(),
            comment: self.comment.map(|comment| HighlightedString::build(comment.to_owned(), pattern, matcher, charbuf)),
            score: self.score,
            label: self.label
        }
    }
}

/// An entry with rich highlight information
/// sent from the plugin manager to the frontend.
#[derive(Debug, Clone)]
pub struct DisplayEntry {
    pub name: HighlightedString,
    pub comment: Option<HighlightedString>,
    pub icon: Option<IconPath>,
    /// fuzzy matching score
    pub score: u32,
    pub label: Label
}

/// A string and information about which parts matchedagainst a pattern
/// and which parts did not.
#[derive(Debug, Clone)]
pub struct HighlightedString {
    source: String,
    /// A list of byte indices indicating alternatively the spans that matched and didn't match
    indices: Vec<u32>
}

impl HighlightedString {
    fn build(source: String, pattern: &Pattern, matcher: &mut Matcher, charbuf: &mut Vec<char>) -> Self {
        let mut indices = vec![];
        pattern.indices(Utf32Str::new(&source, charbuf), matcher, &mut indices);
        indices.sort_unstable();
        indices.dedup();
        Self { source, indices }
    }

    pub fn source(&self) -> &str { &self.source }

    /// Iterate on the highlighted and non highlighted spans
    pub fn iter(&self) -> MatchSpanIterator<'_> {
        let mut chars = self.source.char_indices();
        chars.next(); // advance char iterator to match the state of MatchSpan

        MatchSpanIterator {
            item: &self.source,
            matched: &self.indices,
            matched_index: 0,
            byte_offset: 0,
            index: 0,
            chars
        }
    }
}

use std::str::CharIndices;

pub struct MatchSpanIterator<'a> {
    pub item: &'a str,
    pub matched: &'a [u32],
    pub matched_index: usize,
    pub index: u32,
    pub byte_offset: usize,
    pub chars: CharIndices<'a>
}

impl<'a> Iterator for MatchSpanIterator<'a> {
    type Item = (&'a str, bool);

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.byte_offset;

        let matching = |index, matched_index| Some(&index) == self.matched.get(matched_index);

        // wether or not we start in a matching span 
        let match_state = matching(self.index, self.matched_index);

        // while we are in the same state we were at the beginning
        while matching(self.index, self.matched_index) == match_state {
            if let Some((offset, _)) = self.chars.next() {
                self.byte_offset = offset;
            } else if !self.item[start..].is_empty() {
                self.index += 1;
                self.byte_offset = self.item.len();
                return Some((&self.item[start..], match_state));
            } else {
                // stop when we don't have any characters left
                return None;
            }
            self.index += 1;

            if match_state { self.matched_index += 1 }
        }

        Some((&self.item[start..self.byte_offset], match_state))
    }
}

