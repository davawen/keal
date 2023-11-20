use std::{iter::Peekable, process::{ChildStdin, ChildStdout}, io::{BufRead, BufReader, Write, Lines}, path::{PathBuf, Path}};

use bitflags::bitflags;
use crate::{entries::{EntryTrait, Action}, icon::IconPath};

use super::Plugin;

/// The execution of the currently typed plugin, used for RPC.
/// The underlying process will get killed when it is dropped, so you don't have to fear zombie processes.
#[derive(Debug)]
pub struct PluginExecution {
    pub prefix: String,
    pub entries: Vec<PluginEntry>,
    pub child: std::process::Child,
    stdin: ChildStdin,
    stdout: Peekable<Lines<BufReader<ChildStdout>>>,
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
        let stdout = BufReader::new(stdout).lines().peekable();

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
    pub fn send_query(&mut self, query: &str) -> Action {
        if !self.events.intersects(PluginEvents::Query) { return Action::None }

        writeln!(self.stdin, "query\n{query}").unwrap();
        self.get_action()
    }

    /// Send `enter` event to plugin
    /// Expects `idx` to be valid
    pub fn send_enter(&mut self, idx: usize) -> Action {
        if !self.events.intersects(PluginEvents::Enter) { return Action::None }

        writeln!(self.stdin, "enter\n{idx}").unwrap();
        self.get_action()
    }

    fn get_action(&mut self) -> Action {
        let line = self.stdout.next().unwrap().unwrap();

        match line.split_once(':') {
            Some(("action", action)) => match action.split_once(':') {
                Some(("change_input", value)) => Action::ChangeInput(value.to_owned()),
                Some(("change_query", value)) => Action::ChangeQuery(value.to_owned()),
                Some(("update", index)) => Action::Update(
                    index.parse().unwrap(),
                    self.get_choice_list().pop().expect("one element for update action")
                ),
                _ => match action {
                    "fork" => Action::Fork,
                    "wait_and_close" => Action::WaitAndClose,
                    "update_all" => Action::UpdateAll(self.get_choice_list()),
                    "none" => Action::None,
                    action => panic!("unknown action `{action}`")
                }
            }
            _ => panic!("expected action, got `{line}`")
        }
    }

    fn get_choice_list(&mut self) -> Vec<PluginEntry> {
        let mut entries = vec![];

        // Read initial entries line by line
        while self.stdout.peek().is_some() {
            // looks at the next line
            // if it is "end", or an error, break out of the loop
            match self.stdout.peek().unwrap().as_deref() {
                Ok("end") => {
                    self.stdout.next();
                    break
                }
                Err(_) => break,
                _ => ()
            }

            let (name, icon, comment) = read_entry_from_stream(&mut self.stdout, Some(&self.cwd));
            entries.push(PluginEntry {
                field: name, icon, comment
            });
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
pub struct PluginEntry {
    field: String,
    comment: Option<String>,
    icon: Option<IconPath>
}

impl EntryTrait for PluginEntry {
    fn name(&self) ->  &str { &self.field }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn icon(&self) -> Option<&IconPath> { self.icon.as_ref() }
    fn to_match(&self) ->  &str { &self.field }
}

pub fn read_entry_from_stream<B: BufRead>(
    lines: &mut Peekable<Lines<B>>,
    cwd: Option<&Path>
) -> (String, Option<IconPath>, Option<String>) {
    let (mut name, mut icon, mut comment) = (String::new(), None, None);

    while let Some(line) = lines.next() {
        let Ok(line) = line else { continue };

        match line.split_once(':') {
            Some(("name", n)) => name = n.to_owned(),
            Some(("icon", i)) => icon = Some(IconPath::new(i.to_owned(), cwd)),
            Some(("comment", c)) => comment = Some(c.to_owned()),
            _ if !line.is_empty() => eprintln!("unknown descriptor in input: `{line}`"),
            _ => ()
        }

        if let Some(Ok(next)) = lines.peek() {
            if next.starts_with("name") || next == "end" { break }
        }
    }

    (name, icon, comment)
}
