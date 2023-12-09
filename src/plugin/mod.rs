use std::{process, collections::HashMap};

use crate::{ icon::IconPath, config::Config };
use nucleo_matcher::{Matcher, pattern::Pattern};

pub mod builtin;
pub mod entry;
mod manager;
mod usage;

use self::entry::Entry;
pub use self::manager::{PluginManager, PluginIndex};

pub type PluginGenerator = Box<dyn Fn(&Plugin, &PluginManager) -> Box<dyn PluginExecution> + Send>;
pub struct Plugin {
    pub name: String,
    pub icon: Option<IconPath>,
    pub comment: Option<String>,
    pub prefix: String,
    pub config: HashMap<String, String>,
    pub generator: PluginGenerator
}

pub trait PluginExecution: Send {
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
#[derive(Default, Debug, Clone)]
pub enum Action {
    #[default]
    None,
    // Universal
    ChangeInput(String),
    ChangeQuery(String),
    // Desktop file related
    Exec(ClonableCommand),
    // Dmenu related
    PrintAndClose(String),
    // Plugin related
    Fork,
    WaitAndClose
}

#[derive(Debug)]
pub struct ClonableCommand(pub process::Command);

impl From<process::Command> for ClonableCommand {
    fn from(value: process::Command) -> Self { Self(value) }
}

impl Clone for ClonableCommand {
    fn clone(&self) -> Self {
        let mut c = process::Command::new(self.0.get_program());

        c.args(self.0.get_args())
            .envs(self.0.get_envs().flat_map(|e| Some((e.0, e.1?))));

        if let Some(dir) = self.0.get_current_dir() {
            c.current_dir(dir);
        }

        c.into()
    }
}
