use std::process;

use nucleo_matcher::{Matcher, Utf32Str};
use nucleo_matcher::pattern::Pattern;

use serde::{Serialize, Deserialize};

use crate::config::Config;
use crate::{
    arguments::Arguments, icon::IconPath,
    providers::{
        dmenu::{self, read_dmenu_entries},
        plugin::{execution::{PluginExecution, PluginEntry}, get_plugins, self, Plugins},
        xdg
    }
};

use self::match_span::MatchSpan;
use self::usage::Usage;

mod match_span;
mod usage;

pub trait EntryTrait {
    fn name(&self) -> &str;
    fn comment(&self) -> Option<&str>;
    fn icon(&self) -> Option<&IconPath>;
    /// what should be used to match the entry
    fn to_match(&self) -> &str;

    // TODO: Don't rebuild Utf32Str everytime (it's a lot more convenient for now)
    fn fuzzy_match(&self, matcher: &mut Matcher, pattern: &Pattern, buf: &mut Vec<char>) -> Option<u32> {
        pattern.score(Utf32Str::new(self.to_match(), buf), matcher)
    }

    /// Returns an iterator over the spans of the entry's name that match the given filter
    fn fuzzy_match_span(&self, matcher: &mut Matcher, pattern: &Pattern, buf: &mut Vec<char>) -> MatchSpan {
        let mut indices = vec![];
        pattern.indices(Utf32Str::new(self.name(), buf), matcher, &mut indices);
        indices.sort_unstable();
        indices.dedup();

        let mut chars = self.name().char_indices();
        chars.next(); // advance char iterator to match the state of MatchSpan

        MatchSpan {
            item: self.name(),
            matched: indices,
            matched_index: 0,
            byte_offset: 0,
            index: 0,
            chars
        }
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
pub enum EntryKind {
    Desktop,
    Prefix,
    Plugin,
    Dmenu
}

#[derive(Debug, Clone, Copy)]
pub struct Entry(EntryKind, usize, u32);

#[derive(Default)]
pub struct Entries {
    plugins: Plugins,
    desktop: Vec<xdg::DesktopEntry>,
    prefix: Vec<plugin::PrefixEntry>,
    dmenu: Option<Vec<dmenu::DmenuEntry>>,
    pub execution: Option<PluginExecution>,
    pub filtered: Vec<Entry>,
    usage: Usage
}

impl Entries {
    pub fn new(arguments: &Arguments) -> Self {
        if arguments.dmenu {
            Self {
                dmenu: Some(read_dmenu_entries(arguments.protocol)),
                ..Default::default()
            }
        } else {
            let plugins = get_plugins();

            let current_desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
            let current_desktop: Vec<_> = current_desktop.split(':').collect();

            Self {
                desktop: xdg::desktop_entries(&current_desktop).collect(),
                prefix: plugin::plugin_entries(&plugins).collect(),
                plugins,
                usage: Usage::load(),
                ..Default::default()
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn EntryTrait> {
        self.filtered.iter()
            .map(|&e| self.get_entry(e))
    }

    #[inline]
    fn get_entry(&self, Entry(kind, i, _): Entry) -> &dyn EntryTrait {
        match kind {
            EntryKind::Desktop => &self.desktop[i] as &dyn EntryTrait,
            EntryKind::Prefix => &self.prefix[i] as &dyn EntryTrait,
            EntryKind::Plugin => &self.execution.as_ref().unwrap().entries[i] as &dyn EntryTrait,
            EntryKind::Dmenu => &self.dmenu.as_ref().unwrap()[i] as &dyn EntryTrait
        }
    }

    /// Filters and sorts the `n` closest entries to `filter` into `self.filtered`.
    pub fn filter(&mut self, matcher: &mut Matcher, filter: &Pattern, n: usize, frequency_sort: bool) {
        let mut buf = vec![];
        match &self.execution {
            Some(execution) => {
                self.filtered = fuzzy_match_entries(EntryKind::Plugin, &execution.entries, matcher, filter, &mut buf).collect();
            }
            None => {
                self.filtered = fuzzy_match_entries(EntryKind::Desktop, &self.desktop, matcher, filter, &mut buf).collect();
                self.filtered.extend(fuzzy_match_entries(EntryKind::Prefix, &self.prefix, matcher, filter, &mut buf));
                if let Some(dmenu) = &self.dmenu {
                    self.filtered.extend(fuzzy_match_entries(EntryKind::Dmenu, dmenu, matcher, filter, &mut buf));
                }
            }
        }

        // primary sort ranks by usage
        if frequency_sort {
            // annoying hack needed because `self.get_entry` borrows &self (even though it doesn't use `self.filtered`)
            // this should get optimised away
            let mut filtered = std::mem::take(&mut self.filtered); 

            filtered.sort_by_key(|&entry| 
                std::cmp::Reverse(self.usage.get((entry.0, self.get_entry(entry).name())))
            );
            self.filtered = filtered;
        }

        // secondary sort puts best match at the top (stable = keeps relative order of elements)
        self.filtered.sort_by_key(|&Entry(_, _, score)| std::cmp::Reverse(score));
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
    pub fn launch(&mut self, input: &str, config: &Config, selected: usize) -> Action {
        let Some(&Entry(kind, idx, _)) = self.filtered.get(selected) else { 
            if self.dmenu.is_some() { // return typed text
                return Action::PrintAndClose(input.to_owned());
            }

            return Action::None 
        };

        match kind {
            EntryKind::Desktop => {
                let app = &self.desktop[idx];

                self.usage.add_use((EntryKind::Desktop, app.name()));

                let mut command = if app.terminal {
                    let mut command = process::Command::new(&config.terminal_path);
                    command.arg("-e");
                    command.arg("sh");
                    command
                } else {
                    process::Command::new("sh")
                };
                command.arg("-c").arg(&app.exec);
                if let Some(path) = &app.path {
                    command.current_dir(path);
                }
                Action::Exec(command)
            }
            EntryKind::Prefix => {
                let plugin = &self.prefix[idx];
                self.usage.add_use((EntryKind::Prefix, &plugin.prefix));

                Action::ChangeInput(format!("{} ", plugin.prefix))
            }
            EntryKind::Plugin => {
                let Some(execution) = &mut self.execution else { return Action::None };
                execution.send_enter(idx)
            }
            EntryKind::Dmenu => {
                let dmenu = &self.dmenu.as_ref().unwrap()[idx];
                Action::PrintAndClose(dmenu.name.to_owned())
            }
        }
    }
}

fn fuzzy_match_entries<'a, E: EntryTrait>(kind: EntryKind, entries: &'a [E], matcher: &'a mut Matcher, pattern: &'a Pattern, buf: &'a mut Vec<char>) -> impl Iterator<Item = Entry> + 'a {
    entries.iter()
        .map(|entry| entry.fuzzy_match(matcher, pattern, buf))
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
    // Dmenu related
    PrintAndClose(String),
    // Plugin related
    Fork,
    WaitAndClose,
    UpdateAll(Vec<PluginEntry>),
    Update(usize, PluginEntry),
}
