use std::process;

use crate::{ icon::IconPath, config::Config };
use nucleo_matcher::{Matcher, pattern::Pattern};

pub mod builtin;
pub mod entry;
mod manager;
mod usage;

use self::entry::Entry;
pub use self::manager::{PluginManager, PluginIndex};

pub type PluginGenerator = Box<dyn Fn(&Plugin, &PluginManager) -> Box<dyn PluginExecution>>;
pub struct Plugin {
    pub name: String,
    pub icon: Option<IconPath>,
    pub comment: Option<String>,
    pub prefix: String,
    pub generator: PluginGenerator
}

pub trait PluginExecution {
    /// The plugin is done executing
    fn finished(&mut self) -> bool;
    /// Wait for the plugin to finish executing
    fn wait(&mut self);

    fn send_query(&mut self, config: &Config, query: &str) -> Action;
    fn send_enter(&mut self, config: &Config, query: &str, idx: Option<usize>) -> Action;

    fn get_entries<'a>(&'a self, config: &Config, matcher: &mut Matcher, pattern: &Pattern, out: &mut Vec<Entry<'a>>);

    /// temporary fix for usage frequency: get the name of an entry
    fn get_name(&self, index: usize) -> &str;
}

#[must_use]
#[derive(Default)]
pub enum Action {
    #[default]
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
    WaitAndClose
}
