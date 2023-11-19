use std::process;

use fuzzy_matcher::FuzzyMatcher;

use crate::icon::IconPath;
use crate::providers::plugin::execution::{PluginExecution, PluginEntry};
use crate::providers::{xdg, plugin::{self, Plugins}};

use self::match_span::MatchSpan;

mod match_span;

pub trait EntryTrait<M: FuzzyMatcher> {
    fn name(&self) -> &str;
    fn comment(&self) -> Option<&str>;
    fn icon(&self) -> Option<&IconPath>;
    /// what should be used to match the entry
    fn to_match(&self) -> &str;

    fn fuzzy_match(&self, matcher: &M, filter: &str) -> Option<i64> {
        matcher.fuzzy_match(self.to_match(), filter)
    }

    /// Returns an iterator over the spans of the entry's name that match the given filter
    fn fuzzy_match_span(&self, matcher: &M, filter: &str) -> MatchSpan {
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
#[derive(Debug, Clone, Copy)]
pub enum EntryKind {
    Desktop,
    Prefix,
    Plugin
}

#[derive(Debug, Clone, Copy)]
pub struct Entry(EntryKind, usize, i64);

pub struct Entries {
    plugins: Plugins,
    desktop: Vec<xdg::DesktopEntry>,
    prefix: Vec<plugin::PrefixEntry>,
    pub execution: Option<PluginExecution>,
    pub filtered: Vec<Entry>
}

impl Entries {
    pub fn new(plugins: Plugins) -> Self {
        let current_desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        let current_desktop: Vec<_> = current_desktop.split(':').collect();

        Self {
            desktop: xdg::desktop_entries(&current_desktop).collect(),
            prefix: plugin::plugin_entries(&plugins).collect(),
            plugins,
            execution: None,
            filtered: vec![]
        }
    }

    pub fn iter<M: FuzzyMatcher>(&self) -> impl Iterator<Item = &dyn EntryTrait<M>> {
        self.filtered.iter()
            .map(|&Entry(kind, i, _)| match kind {
                EntryKind::Desktop => &self.desktop[i] as &dyn EntryTrait<M>,
                EntryKind::Prefix => &self.prefix[i] as &dyn EntryTrait<M>,
                EntryKind::Plugin => &self.execution.as_ref().unwrap().entries[i] as &dyn EntryTrait<M>
            })
    }

    /// Filters and sorts the `n` closest entries to `filter` into `self.filtered`.
    pub fn filter(&mut self, matcher: &impl FuzzyMatcher, filter: &str, n: usize) {
        match &self.execution {
            Some(execution) => {
                self.filtered = fuzzy_match_entries(matcher, EntryKind::Plugin, &execution.entries, filter).collect();
            }
            None => {
                self.filtered = fuzzy_match_entries(matcher, EntryKind::Desktop, &self.desktop, filter)
                    .chain(fuzzy_match_entries(matcher, EntryKind::Prefix, &self.prefix, filter))
                    .collect()
            }
        }

        self.filtered.sort_unstable_by_key(|&Entry(_, _, score)| std::cmp::Reverse(score));
        self.filtered.truncate(n);
    }

    /// Changes the input field to a new value
    /// `from_user` describes wether this change originates from user interaction
    /// Or wether it comes from a plugin action, (and should therefore not be propagated as an event, to avoid cycles).
    /// Returns the actual query string, and the action that resulted from the input
    /// Important: remember to call `filter` after calling this function
    pub fn update_input(&mut self, input: &str, from_user: bool) -> (String, Action) {
        // launch or stop plugin execution depending on new state of filter
        // if in plugin mode, remove plugin prefix from filter
        let (query, action) = match (self.plugins.filter_starts_with_plugin(input), &mut self.execution) {
            (Some((plugin, remainder)), None) => { // launch plugin
                self.execution = Some(plugin.generate());
                (remainder.to_owned(), Action::None)
            }
            (Some((plugin, remainder)), Some(execution)) => {
                let remainder = remainder.to_owned();

                // relaunch plugin if it is done executing or if we're currently executing the wrong plugin
                if execution.child.try_wait().unwrap().is_some() || plugin.prefix != execution.prefix {
                    *execution = plugin.generate();
                } else if from_user { // send query event
                    let action = execution.send_query(&remainder);
                    return (remainder, action);
                }

                (remainder, Action::None)
            }
            (None, Some(_)) => { // stop plugin
                self.execution = None;
                (input.to_owned(), Action::None)
            }
            (None, None) => (input.to_owned(), Action::None)
        };

        (query, action)
    }

    /// `selected` is an index into `self.filtered`. it may not be valid.
    pub fn launch(&mut self, selected: usize) -> Action {
        let Some(&Entry(kind, idx, _)) = self.filtered.get(selected) else { return Action::None };

        match kind {
            EntryKind::Desktop => {
                let app = &self.desktop[idx];

                let mut command = process::Command::new("sh"); // ugly work around to avoir parsing spaces/quotes
                command.arg("-c").arg(&app.exec);
                if let Some(path) = &app.path {
                    command.current_dir(path);
                }
                Action::Exec(command)
            }
            EntryKind::Prefix => {
                let plugin = &self.prefix[idx];
                Action::ChangeInput(format!("{} ", plugin.prefix))
            }
            EntryKind::Plugin => {
                let Some(execution) = &mut self.execution else { return Action::None };
                execution.send_enter(idx)
            }
        }
    }
}

fn fuzzy_match_entries<'a, M: FuzzyMatcher, E: EntryTrait<M>>(matcher: &'a M, kind: EntryKind, entries: &'a [E], filter: &'a str) -> impl Iterator<Item = Entry> + 'a {
    entries.iter()
        .map(|entry| entry.fuzzy_match(matcher, filter))
        .enumerate()
        .flat_map(move |(i, e)| Some(Entry(kind, i, e?)))
}

#[must_use]
pub enum Action {
    None,
    // Universal
    ChangeInput(String),
    ChangeQuery(String),
    // Desktop file related
    Exec(process::Command),
    // Plugin related
    Fork,
    WaitAndClose,
    UpdateAll(Vec<PluginEntry>),
    Update(usize, PluginEntry),
}
