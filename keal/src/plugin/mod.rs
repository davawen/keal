use std::{process, sync::mpsc};

use crate::{ icon::IconPath, config::Config };
use entry::{Label, DisplayEntry};
use fork::{fork, Fork};
use indexmap::IndexMap;
use nucleo_matcher::{Matcher, pattern::Pattern};

mod builtin;
pub mod entry;
mod manager;
mod usage;

use self::entry::Entry;
use self::manager::{PluginManager, PluginIndex};

type PluginGenerator = Box<dyn Fn(&Plugin, &PluginManager) -> Box<dyn PluginExecution> + Send>;

struct Plugin {
    pub name: String,
    pub icon: Option<IconPath>,
    pub comment: Option<String>,
    pub prefix: String,
    pub config: IndexMap<String, String>,
    generator: PluginGenerator
}

trait PluginExecution: Send {
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

#[must_use]
#[derive(Default, Debug, Clone)]
enum Action {
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

#[derive(Debug, Clone)]
pub enum FrontendAction {
    UpdateEntries { entries: Vec<DisplayEntry>, query: String },
    ChangeInput(String),
    Close
}

#[derive(Debug, Clone)]
pub enum FrontendEvent {
    UpdateInput { input: String, from_user: bool },
    Launch(Option<Label>)
}

/// Launch the keal plugin manager on another thread,
/// and create the necessary communication bits
pub fn init(num_entries: usize, sort_by_usage: bool) -> (mpsc::Sender<FrontendEvent>, mpsc::Receiver<FrontendAction>) {
    let (event_sx, event_rx) = mpsc::channel();
    let (action_sx, action_rx) = mpsc::channel();

    std::thread::spawn(move || {
        let (event_rx, action_sx) = (event_rx, action_sx);
        let mut manager = PluginManager::default();
        manager.load_plugins();

        let mut query = String::new();
        let mut matcher = Matcher::default();
        let mut pattern = Pattern::default();

        let send_action_to_frontend = |action: Action, manager: &mut PluginManager| {
            let action = match action {
                Action::None => return,
                Action::ChangeInput(input) => {
                    manager.kill();
                    FrontendAction::ChangeInput(input)
                },
                Action::ChangeQuery(query) => {
                    let input = manager.current().map(|plugin| format!("{} {}", plugin.prefix, query)).unwrap_or(query);
                    FrontendAction::ChangeInput(input)
                }
                Action::Exec(mut command) => {
                    use std::os::unix::process::CommandExt;
                    let _ = command.0.exec();
                    FrontendAction::Close
                }
                Action::Fork => match fork().expect("failed to fork") {
                    Fork::Parent(_) => FrontendAction::Close,
                    Fork::Child => return
                },
                Action::PrintAndClose(message) => {
                    println!("{message}");
                    FrontendAction::Close
                }
                Action::WaitAndClose => {
                    manager.wait();
                    FrontendAction::Close
                }
            };
            let _ = action_sx.send(action);
        };

        loop {
            let event = match event_rx.recv() {
                Ok(event) => event,
                Err(_) => break,
            };

            match event {
                FrontendEvent::UpdateInput { input, from_user } => {
                    let (new_query, action) = manager.update_input(&input, from_user);
                    query = new_query;
                    pattern.reparse(&query, nucleo_matcher::pattern::CaseMatching::Ignore, nucleo_matcher::pattern::Normalization::Smart);

                    let entries = manager.get_entries(&mut matcher, &pattern, num_entries, sort_by_usage);

                    let _ = action_sx.send(FrontendAction::UpdateEntries { entries, query: query.clone() });
                    send_action_to_frontend(action, &mut manager);
                }
                FrontendEvent::Launch(label) => {
                    let action = manager.launch(&query, label);
                    send_action_to_frontend(action, &mut manager);
                }
            }
        }
    });

    (event_sx, action_rx)
}
