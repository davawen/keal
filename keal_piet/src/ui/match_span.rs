use std::str::CharIndices;

use nucleo_matcher::{Matcher, pattern::Pattern, Utf32Str};

pub struct MatchSpan<'a> {
    pub item: &'a str,
    pub matched: Vec<u32>,
    pub matched_index: usize,
    pub index: u32,
    pub byte_offset: usize,
    pub chars: CharIndices<'a>
}

impl<'a> MatchSpan<'a> {
    pub fn new(item: &'a str, matcher: &mut Matcher, pattern: &Pattern, charbuf: &mut Vec<char>) -> Self {
        let mut indices = vec![];
        pattern.indices(Utf32Str::new(item, charbuf), matcher, &mut indices);
        indices.sort_unstable();
        indices.dedup();

        let mut chars = item.char_indices();
        chars.next(); // advance char iterator to match the state of MatchSpan

        MatchSpan {
            item,
            matched: indices,
            matched_index: 0,
            byte_offset: 0,
            index: 0,
            chars
        }
    }
}

impl<'a> Iterator for MatchSpan<'a> {
    type Item = (std::ops::Range<usize>, bool);

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
                return Some((start..self.item.len(), match_state));
            } else {
                // stop when we don't have any characters left
                return None;
            }
            self.index += 1;

            if match_state { self.matched_index += 1 }
        }

        Some((start..self.byte_offset, match_state))
    }
}

