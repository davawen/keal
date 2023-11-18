use std::{process::{ChildStdin, ChildStdout}, io::{BufRead, BufReader, Write}, path::PathBuf};

use bitflags::bitflags;

use crate::{search::{EntryTrait, Entry}, icon::IconPath};

use super::Plugin;

/// The execution of the currently typed plugin, used for RPC.
/// The underlying process will get killed when it is dropped, so you don't have to fear zombie processes.
#[derive(Debug)]
pub struct PluginExecution {
    pub prefix: String,
    pub entries: Vec<Entry>,
    pub child: std::process::Child,
    stdin: ChildStdin,
    stdout: std::io::Lines<BufReader<ChildStdout>>,
    events: PluginEvents,
    cwd: PathBuf
}

bitflags! {
    #[derive(Debug)]
    struct PluginEvents: u8 {
        const None = 0;
        const Enter = 0b1;
        const ShiftEnter = 0b10;
        const Query = 0b100;
    }
}

#[must_use]
pub enum PluginAction {
    Fork,
    WaitAndClose,
    ChangeInput(String),
    ChangeQuery(String),
    UpdateAll(Vec<Entry>),
    Update(usize, Entry),
    None
}

// TODO: Better error handling for plugins: instead of panicking or logging to stderr, show feedback in window
// TODO: Asynchronous/Non blocking plugins

impl PluginExecution {
    pub fn new(plugin: &Plugin) -> Self {
        use std::process::{Stdio, Command};

        let cwd = plugin.exec.parent().unwrap().to_path_buf();
        let mut child = Command::new(&plugin.exec)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .current_dir(&cwd)
            .spawn().expect("Couldn't spawn process from plugin");

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stdout = BufReader::new(stdout).lines();

        let mut this = Self {
            prefix: plugin.prefix.clone(),
            child, stdin, stdout,
            entries: vec![],
            events: PluginEvents::None,
            cwd
        };

        this.get_events();
        this.entries = this.get_choice_list();

        this
    }

    fn get_events(&mut self) {
        let line = self.stdout.next().unwrap().unwrap();

        match line.split_once(':') {
            Some(("events", events)) => for event in events.split(' ') {
                match event {
                    "enter" => self.events |= PluginEvents::Enter,
                    "shift-enter" => self.events |= PluginEvents::ShiftEnter,
                    "query" => self.events |= PluginEvents::Query,
                    event => panic!("unknown event `{event}`")
                }
            }
            _ => panic!("expected subscribed events, got `{line}`") // Perhaps we can assume enter?
        }
    }

    /// Send `query` event to plugin
    pub fn send_query(&mut self, query: &str) -> Option<PluginAction> {
        if !self.events.intersects(PluginEvents::Query) { return None }

        writeln!(self.stdin, "query\n{query}").unwrap();
        Some(self.get_action())
    }

    /// Send `enter` event to plugin
    /// Expects `idx` to be valid
    pub fn send_enter(&mut self, idx: usize) -> Option<PluginAction> {
        if !self.events.intersects(PluginEvents::Enter) { return None }

        writeln!(self.stdin, "enter\n{idx}").unwrap();
        Some(self.get_action())
    }

    fn get_action(&mut self) -> PluginAction {
        let line = self.stdout.next().unwrap().unwrap();

        match line.split_once(':') {
            Some(("action", action)) => match action.split_once(':') {
                Some(("change_input", value)) => PluginAction::ChangeInput(value.to_owned()),
                Some(("change_query", value)) => PluginAction::ChangeQuery(value.to_owned()),
                Some(("update", index)) => PluginAction::Update(
                    index.parse().unwrap(),
                    self.get_choice_list().pop().expect("one element for update action")
                ),
                _ => match action {
                    "fork" => PluginAction::Fork,
                    "wait_and_close" => PluginAction::WaitAndClose,
                    "update_all" => PluginAction::UpdateAll(self.get_choice_list()),
                    "none" => PluginAction::None,
                    action => panic!("unknown action `{action}`")
                }
            }
            _ => panic!("expected action, got `{line}`")
        }
    }

    fn get_choice_list(&mut self) -> Vec<Entry> {
        let mut entries = vec![];
        let mut current: Option<FieldEntry> = None;

        // Read initial entries line by line
        for line in self.stdout.by_ref() {
            let Ok(line) = line else { continue };
            if line.is_empty() { continue }

            match line.split_once(':') {
                Some(line) => match (&mut current, line) {
                    (current, ("name", name)) => {
                        if let Some(old) = current.take() {
                            entries.push(old.into());
                        }

                        *current = Some(FieldEntry {
                            field: name.to_owned(),
                            icon: None,
                            comment: None
                        });
                    }
                    (Some(current), ("icon", icon)) => current.icon = Some(IconPath::new(icon.to_owned(), Some(&self.cwd))),
                    (Some(current), ("comment", comment)) => current.comment = Some(comment.to_owned()),
                    (None, ("icon" | "comment", _)) => eprintln!("using a modifier descriptor before setting a field in plugin `{}`", self.prefix),
                    (_, (descriptor, _)) => eprintln!("unknown descriptor `{descriptor}` in plugin `{}`", self.prefix)
                },
                None => match line.as_str() {
                    "end" => {
                        if let Some(old) = current.take() {
                            entries.push(old.into());
                        }
                        break
                    }
                    line => eprintln!("expected choice or `end`, got `{line}`.")
                }
            }
        }

        entries
    }
}

impl Drop for PluginExecution {
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => (), // process has already exited
            _ => {
                let _ = self.child.kill(); // ignore any resulting error
            }
        }
    }
}

#[derive(Debug)]
pub struct FieldEntry {
    field: String,
    comment: Option<String>,
    icon: Option<IconPath>
}

impl EntryTrait for FieldEntry {
    fn name(&self) ->  &str { &self.field }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn icon(&self) -> Option<&IconPath> { self.icon.as_ref() }
    fn to_match(&self) ->  &str { &self.field }
}
