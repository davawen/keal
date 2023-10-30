use std::collections::HashMap;

use enum_dispatch::enum_dispatch;
use fuzzy_matcher::FuzzyMatcher;

use self::{match_span::MatchSpan, xdg::DesktopEntry, plugin::PluginEntry};

pub mod xdg;
mod plugin;
mod match_span;

#[enum_dispatch]
pub trait EntryTrait {
    fn name(&self) -> &str;
    fn comment(&self) -> Option<&str>;
    /// what should be used to match the entry
    fn to_match(&self) -> &str;

    fn fuzzy_match(&self, matcher: &impl FuzzyMatcher, filter: &str) -> Option<i64> {
        matcher.fuzzy_match(self.to_match(), filter)
    }

    /// Returns an iterator over the spans of the entry's name that match the given filter
    fn fuzzy_match_span(&self, matcher: &impl FuzzyMatcher, filter: &str) -> MatchSpan {
        let mut chars = self.name().char_indices();
        chars.next(); // advance char iterator to match the state of MatchSpan

        MatchSpan {
            item: self.name(),
            matched: matcher.fuzzy_indices(self.name(), filter).map(|(_, v)| v).unwrap_or(vec![]),
            matched_index: 0,
            byte_offset: 0,
            index: 0,
            chars
        }
    }
}

#[enum_dispatch(EntryTrait)]
pub enum Entry {
    DesktopEntry,
    PluginEntry
}

pub fn create_entries() -> Vec<Entry> {
    xdg::desktop_entries().map(Entry::from)
        .chain(plugin::plugin_entries().map(Entry::from))
        .collect()
}

/// Returns the `n` closest entries to the `filter`, as a Vec of indices into the original slice, sorted in descending order.
/// Second tuple element is the score.
pub fn filter_entries(matcher: &impl FuzzyMatcher, entries: &[Entry], filter: &str, n: usize) -> Vec<(usize, i64)> {
    let mut filtered: Vec<_> = entries.iter()
        .map(|entry| entry.fuzzy_match(matcher, filter))
        .enumerate()
        .flat_map(|(i, score)| score.map(|s| (i, s)))
        .collect();

    filtered.sort_unstable_by_key(|&(_, score)| std::cmp::Reverse(score));
    filtered.truncate(n);
    filtered
}

/// Links an icon name to its path
#[derive(Debug)]
pub struct IconCache(HashMap<String, String>);

fn icon_cache(icon_theme: &str) -> IconCache {
    // let mut icon_dirs = xdg_directories("icons");
    // icon_dirs.push("/usr/share/pixmaps".to_owned());

    todo!()
}
