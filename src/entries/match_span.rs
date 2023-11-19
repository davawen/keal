use std::str::CharIndices;


pub struct MatchSpan<'a> {
    pub item: &'a str,
    pub matched: Vec<usize>,
    pub matched_index: usize,
    pub index: usize,
    pub byte_offset: usize,
    pub chars: CharIndices<'a>
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

