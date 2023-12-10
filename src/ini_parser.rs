//! Api design inspired by [tini](https://github.com/pinecrew/tini)

use std::{collections::HashMap, path::Path};

use indexmap::IndexMap;

#[derive(Debug, Default)]
pub struct Section {
    keys: IndexMap<String, String>
}

impl Section {
    pub fn into_map(self) -> IndexMap<String, String> {
        self.keys
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.keys.iter()
    }
}

impl IntoIterator for Section {
    type Item = (String, String);
    type IntoIter = indexmap::map::IntoIter<String, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.keys.into_iter()
    }

}

#[derive(Debug)]
pub struct Ini {
    globals: IndexMap<String, String>,
    sections: HashMap<String, Section>
}

impl Ini {
    pub fn from_file<P: AsRef<Path>>(path: P, comment_chars: &[char]) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_string(content, comment_chars))
    }

    /// `comment_chars`: which characters start line comments?
    pub fn from_string(file: String, comment_chars: &[char]) -> Self {
        let mut this = Self {
            globals: IndexMap::default(),
            sections: HashMap::default()
        };

        let mut current_section = None;
        for line in file.lines() {
            this.parse_line(&mut current_section, line, comment_chars);
        }

        if let Some((name, section)) = current_section {
            this.insert(name, section);
        }

        this
    }

    fn parse_line(&mut self, current_section: &mut Option<(String, Section)>, line: &str, comment_chars: &[char]) {
        let content = match line.split(comment_chars).next() {
            Some(content) => content.trim(),
            None => return
        };

        if content.is_empty() {
            return
        }

        if content.starts_with('[') && content.ends_with(']') {
            if let Some(section) = current_section.take() {
                self.insert(section.0, section.1);
            }

            *current_section = Some((content[1..content.len()-1].to_owned(), Section::default()));
        } else if let Some((name, value)) = content.split_once('=') {
            let keys = current_section.as_mut().map(|(_, section)| &mut section.keys).unwrap_or(&mut self.globals);
            keys.insert(name.trim().to_owned(), value.trim().to_owned());
        }
    }

    fn insert(&mut self, name: String, section: Section) {
        self.sections.insert(name, section);
    }

    #[allow(unused)]
    pub fn globals(&self) -> impl Iterator<Item = (&String, &String)> {
        self.globals.iter()
    }

    pub fn into_sections(self) -> impl Iterator<Item = (String, Section)> {
        self.sections.into_iter()
    }

    /// Returns an empty iterator if section does not exist
    pub fn section_iter(&self, section: &str) -> impl Iterator<Item = (&String, &String)> {
        self.section(section).into_iter().flat_map(|s| s.iter())
    }

    pub fn section(&self, section: &str) -> Option<&Section> {
        self.sections.get(section)
    }

    pub fn remove_section(&mut self, section: &str) -> Option<Section> {
        self.sections.remove(section)
    }
}
