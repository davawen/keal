use std::{borrow::Borrow, hash::Hash, collections::HashMap, path::PathBuf};
use serde::{Serialize, Deserialize};

use super::EntryKind;

// type nonsense to allow borrowing the string that goes in the key
trait UsageKey {
    fn a(&self) -> EntryKind;
    fn b(&self) -> &str;
}

impl<'a> Borrow<dyn UsageKey + 'a> for (EntryKind, String) {
    fn borrow(&self) -> &(dyn UsageKey + 'a) {
        self
    }
}

impl Hash for dyn UsageKey + '_ {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.a().hash(state);
        self.b().hash(state);
    }
}

impl PartialEq for dyn UsageKey + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.a() == other.a() && self.b() == other.b()
    }
}
impl Eq for dyn UsageKey + '_ {}

impl UsageKey for (EntryKind, String) {
    fn a(&self) -> EntryKind { self.0 }
    fn b(&self) -> &str { &self.1 }
}

impl<'a> UsageKey for (EntryKind, &'a str) {
    fn a(&self) -> EntryKind { self.0 }
    fn b(&self) -> &'a str { self.1 }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Usage(HashMap<(EntryKind, String), usize>);

impl Usage {
    /// Gets the canonical file path to the usage file
    /// NOTE: this creates the state directory if it doesn't exist!
    fn file_path() -> PathBuf {
        use crate::xdg_utils::state_dir;
        let mut path = state_dir().unwrap();
        let _ = std::fs::create_dir_all(&path);

        path.push("usage.cbor");
        path
    }

    pub fn load() -> Self {
        let usage = Usage::file_path();
        if let Ok(file) = std::fs::File::open(&usage) {
            serde_cbor::from_reader(file).unwrap_or_else(|_| {
                // assume corrupted file
                let _ = std::fs::remove_file(&usage);
                Usage::default()
            })
        } else { Usage::default() }
    }

    #[inline(always)]
    pub fn get(&self, k: (EntryKind, &str)) -> Option<&usize> {
        self.0.get(&k as &dyn UsageKey)
    }

    /// Adds one use to a given entry (and saves it to disk)
    /// If it doesn't exist, this inserts it and sets its count to 1 (by cloning the input `&str`)
    pub fn add_use(&mut self, k: (EntryKind, &str)) {
        if let Some(v) = self.0.get_mut(&k as &dyn UsageKey) {
            *v += 1;
        } else {
            self.0.insert((k.0, k.1.to_owned()), 1);
        }

        let usage = Usage::file_path();
        let file = std::fs::File::create(usage).expect("failed to write to usage file");
        let _ = serde_cbor::to_writer(file, self);
    }
}
