use enum_dispatch::enum_dispatch;
use fuzzy_matcher::FuzzyMatcher;

use self::{match_span::MatchSpan, xdg::DesktopEntry, plugin::{PluginEntry, Plugins, FieldEntry}};

pub mod xdg;
pub mod plugin;
mod match_span;

#[enum_dispatch]
pub trait EntryTrait {
    fn name(&self) -> &str;
    fn comment(&self) -> Option<&str>;
    fn icon(&self) -> Option<&str>;
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
            matched: matcher.fuzzy_indices(self.name(), filter).map(|(_, v)| v).unwrap_or_default(),
            matched_index: 0,
            byte_offset: 0,
            index: 0,
            chars
        }
    }
}

#[allow(clippy::enum_variant_names)]
#[enum_dispatch(EntryTrait)]
#[derive(Debug)]
pub enum Entry {
    DesktopEntry,
    PluginEntry,
    FieldEntry
}

pub fn create_entries(plugins: &Plugins) -> Vec<Entry> {
    xdg::desktop_entries().map(Entry::from)
        .chain(plugin::plugin_entries(plugins).map(Entry::from))
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
