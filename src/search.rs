use std::{fs, collections::HashMap, str::CharIndices};

use fuzzy_matcher::FuzzyMatcher;
use tini::Ini;

fn xdg_directories(dir: &str) -> Vec<String> {
    let mut data_dirs = std::env::var("XDG_DATA_DIRS").map(|dirs| dirs.split(':').map(str::to_owned).collect()).unwrap_or(vec![]);
    if let Ok(home) = std::env::var("HOME") {
        data_dirs.push(format!("{home}/.local/share"));
    }

    data_dirs.into_iter().map(|path| format!("{path}/{dir}")).collect()
}

#[derive(Debug)]
pub struct DesktopEntry {
    pub name: String,
    comment: Option<String>,
    /// string that will be used for fuzzy matching
    /// concatenation of name, generic name, categories and comment
    to_match: String
}

impl DesktopEntry {
    fn new(mut ini: HashMap<String, String>) -> Self {
        let name = ini.remove("Name").unwrap();
        let comment = ini.remove("Description");
        DesktopEntry {
            to_match: format!(
                "{name}{}{}{}",
                ini.get("GenericName").map(String::as_ref).unwrap_or(""),
                ini.get("Categories").map(String::as_ref).unwrap_or(""),
                comment.as_deref().unwrap_or(""),
            ),
            name,
            comment
        }
    }

    pub fn fuzzy_match(&self, matcher: &impl FuzzyMatcher, filter: &str) -> Option<i64> {
        matcher.fuzzy_match(&self.to_match, filter)
    }

    /// Returns an iterator over the spans of the entry's name that match the given filter
    pub fn fuzzy_name_span(&self, matcher: &impl FuzzyMatcher, filter: &str) -> MatchSpan {
        let mut chars = self.name.char_indices();
        chars.next(); // advance char iterator to match the state of MatchSpan

        MatchSpan {
            item: &self.name,
            matched: matcher.fuzzy_indices(&self.name, filter).map(|(_, v)| v).unwrap_or(vec![]),
            matched_index: 0,
            byte_offset: 0,
            index: 0,
            chars
        }
    }
}

pub struct MatchSpan<'a> {
    item: &'a str,
    matched: Vec<usize>,
    matched_index: usize,
    index: usize,
    byte_offset: usize,
    chars: CharIndices<'a>
}

impl<'a> Iterator for MatchSpan<'a> {
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

/// Returns the list of all applications on the system
pub fn desktop_entries() -> Vec<DesktopEntry> {
    let app_dirs = xdg_directories("applications");

    let mut values = vec![];
    for path in app_dirs {
        let entries = fs::read_dir(path);
        let Ok(entries) = entries else { continue };

        let entries = entries
            .flatten()
            .filter(|entry| entry.metadata().map(|x| !x.is_dir()).unwrap_or(true))
            .map(|entry| entry.path())
            .filter(|path| path.extension().map(|e| e == "desktop").unwrap_or(false))
            .flat_map(|path| Ini::from_file(&path))
            .map(|ini| ini.section_iter("Desktop Entry").map(|(a, b)| (a.to_owned(), b.to_owned())).collect::<HashMap<_, _>>())
            .map(DesktopEntry::new);
        
        values.extend(entries);
    }

    values
}

/// Returns the `n` closest entries to the `filter`, as a Vec of indices into the original slice, sorted in descending order.
/// Second tuple element is the score.
pub fn filter_entries(matcher: &impl FuzzyMatcher, entries: &[DesktopEntry], filter: &str, n: usize) -> Vec<(usize, i64)> {
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
    let mut icon_dirs = xdg_directories("icons");
    // icon_dirs.push("/usr/share/pixmaps".to_owned());



    todo!()
}
