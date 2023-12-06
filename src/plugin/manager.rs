use indexmap::IndexMap;
use nucleo_matcher::{Matcher, pattern::Pattern};

use crate::{config::Config, arguments::Arguments};

use super::{Plugin, PluginExecution, builtin::{user::get_user_plugins, application::ApplicationPlugin}, Action, usage::Usage, entry::{Label, OwnedEntry}};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PluginIndex(usize);

#[derive(Default)]
pub struct PluginManager {
    /// the list of all loaded plugins
    plugins: IndexMap<String, Plugin>,
    /// plugins selected by default by the user that will show when no plugin prefix is typed
    default_plugins: Vec<(PluginIndex, Box<dyn PluginExecution>)>,
    /// if the user has typed a plugin prefix, then this will be the only plugin shown
    /// usize is an index into `self.plugins`
    current: Option<(PluginIndex, Box<dyn PluginExecution>)>,
    /// how frequently different plugin entries are used
    usage: Usage
}

impl PluginManager {
    pub fn load_plugins(&mut self, arguments: &Arguments) {
        if arguments.dmenu {
            let dmenu = super::builtin::dmenu::DmenuPlugin::create(arguments.protocol);
            self.plugins = IndexMap::from_iter([
                (dmenu.prefix.clone(), dmenu)
            ]);
            // add dmenu to default plugins at startup
            self.add_default_plugin(0);
        } else {
            self.plugins = get_user_plugins().into_iter().flatten().collect();
            self.usage = Usage::load();

            let current_desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
            let applications = ApplicationPlugin::create(current_desktop);
            self.plugins.insert(applications.prefix.clone(), applications);
            self.add_default_plugin(self.plugins.len() - 1);
        }
    }

    fn add_default_plugin(&mut self, index: usize) {
        let plugin = &self.plugins[index];
        self.default_plugins.push((PluginIndex(index), (plugin.generator)(plugin, self)));
    }

    pub fn get_entries(&self, config: &Config, matcher: &mut Matcher, pattern: &Pattern, n: usize, sort_by_usage: bool) -> Vec<OwnedEntry> {
        let mut entries = vec![];
        let mut buf = vec![];
        if let Some((idx, current)) = &self.current {
            current.get_entries(config, matcher, pattern, &mut buf);
            entries.extend(buf.drain(..).map(|e| e.label(*idx)));
        } else {
            for (idx, plug) in &self.default_plugins {
                plug.get_entries(config, matcher, pattern, &mut buf);
                entries.extend(buf.drain(..).map(|e| e.label(*idx)));
            }
        }

        // primary sort ranks by usage
        if sort_by_usage {
            entries.sort_by_key(|entry|
                std::cmp::Reverse(self.usage.get((&self.plugins[entry.label.plugin_index.0].name, &entry.name)))
            );
        }

        // secondary sort puts best match at the top (stable = keeps relative order of elements)
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.score));
        entries.truncate(n);

        // this clones the value of only the top keys, which should incur pretty minimal performance loss
        // in response, it allows putting plugins in an async future, which is a much bigger win than a few avoided clones
        entries.into_iter().map(|e| e.to_owned()).collect()
    }

    /// Changes the input field to a new value
    /// `from_user` describes wether this change originates from user interaction
    /// Or wether it comes from a plugin action, (and should therefore not be propagated as an event, to avoid cycles).
    /// Returns the actual query string, and the action that resulted from the input
    pub fn update_input(&mut self, config: &Config, input: &str, from_user: bool) -> (String, Action) {
        let filter_starts_with_plugin = if let Some((name, remainder)) = input.split_once(' ') {
            self.plugins.get_full(name).map(|(idx, _, plugin)| ((PluginIndex(idx), plugin), remainder))
        } else { None };

        // launch or stop plugin execution depending on new state of filter
        // if in plugin mode, remove plugin prefix from filter
        let (query, action) = match (filter_starts_with_plugin, &mut self.current) {
            (Some(((idx, plugin), remainder)), None) => { // launch plugin
                let execution = (plugin.generator)(plugin, self);
                self.current = Some((idx, execution));

                (remainder.to_owned(), Action::None)
            }
            (Some(((idx, plugin), remainder)), Some((execution_idx, execution))) => {
                let remainder = remainder.to_owned();

                // relaunch plugin if it is done executing or if we're currently executing the wrong plugin
                if execution.finished() || idx != *execution_idx {
                    let execution = (plugin.generator)(plugin, self);
                    self.current = Some((idx, execution));
                } else if from_user { // send query event
                    let action = execution.send_query(config, &remainder);
                    return (remainder, action);
                }

                (remainder, Action::None)
            }
            (None, Some(_)) => { // stop plugin
                self.current = None;
                (input.to_owned(), Action::None)
            }
            (None, None) => (input.to_owned(), Action::None)
        };

        (query, action)
    }

    /// `selected` contains the `plugin_idx` field of a `LabelledEntry`, and the `index` field of an `Entry`
    pub fn launch(&mut self, config: &Config, query: &str, selected: Option<Label>) -> Action {
        if let Some((plug, current)) = &mut self.current {
            if let Some(Label { index, .. }) = selected {
                self.usage.add_use((&self.plugins[plug.0].name, current.get_name(index)));
            }

            current.send_enter(config, query, selected.map(|s| s.index))
        } else if let Some(Label { plugin_index, index }) = selected {
            if let Some((_, execution)) = self.default_plugins.iter_mut().find(|(idx, _)| *idx == plugin_index) {
                self.usage.add_use((&self.plugins[plugin_index.0].name, execution.get_name(index)));
                execution.send_enter(config, query, Some(index))
            } else { Action::None }
        } else { Action::None }
    }

    /// kills current running plugin
    pub fn kill(&mut self) {
        self.current = None;
    }

    /// gets the plugin reference of the currently running execution
    pub fn current(&self) -> Option<&Plugin> {
        self.current.as_ref().map(|(idx, _)| self.plugins.get_index(idx.0).unwrap().1)
    }

    /// wait for the current plugin to finish executing
    pub fn wait(&mut self) {
        if let Some((_, execution)) = &mut self.current {
            execution.wait();
        }
    }
}
