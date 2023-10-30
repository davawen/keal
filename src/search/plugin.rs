use std::{collections::HashMap, io::{BufRead, BufReader}, process::{ChildStdout, ChildStdin}};

use super::EntryTrait;

#[derive(Clone)]
pub struct Plugin {
    pub prefix: String,
    pub comment: Option<String>,
    pub exec: String
}

/// The execution of the currently typed plugin, used for RPC.
/// The underlying process will get killed when it is dropped, so you don't have to fear zombie processes.
#[derive(Debug)]
pub struct PluginExecution {
    pub prefix: String,
    pub child: std::process::Child,
    pub stdin: ChildStdin,
    pub stdout: std::io::Lines<BufReader<ChildStdout>>,
    pub entries: Vec<super::Entry>
}

impl Plugin {
    pub fn generate(&self) -> PluginExecution {
        use std::process::{Stdio, Command};

        let mut child = Command::new(&self.exec)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn().expect("Couldn't spawn process from plugin");

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut stdout = std::io::BufReader::new(stdout).lines();

        let mut entries = vec![];
        let mut current: Option<FieldEntry> = None;

        for line in stdout.by_ref() {
            let Ok(line) = line else { continue };

            match (&mut current, line.get(0..2)) {
                (old, Some("F:")) => {
                    if let Some(old) = old.take() {
                        entries.push(old.into());
                    }
                    current = Some(FieldEntry {
                        field: line[2..].to_owned(),
                        icon: None,
                        comment: None
                    });
                }
                (Some(ref mut current), Some("I:")) => current.icon = Some(line[2..].to_owned()),
                (Some(ref mut current), Some("C:")) => current.comment = Some(line[2..].to_owned()),
                (current, Some("E:")) => {
                    if let Some(current) = current.take() { entries.push(current.into()) }
                    break
                }
                (None, Some("I:" | "C:")) => eprintln!("using a modifier descriptor before setting a field in plugin `{}`", self.prefix),
                (_, Some(d)) => eprintln!("unknown descriptor `{d}` in plugin `{}`", self.prefix),
                (_, None) => eprintln!("no descriptor given in plugin `{}`", self.prefix)
            }
        }

        PluginExecution {
            prefix: self.prefix.clone(),
            child, stdin, stdout, entries
        }
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

pub struct Plugins(HashMap<String, Plugin>);

impl Plugins {
    /// If filter starts with a plugin name and a space, returns the given plugin and the remainder of the string to fuzzy match.
    pub fn filter_starts_with_plugin<'a, 'b>(&'a self, filter: &'b str) -> Option<(&'a Plugin, &'b str)> {
        let (name, remainder) = filter.split_once(' ')?;
        let plugin = self.0.get(name)?;

        Some((plugin, remainder))
    }
}

pub fn get_plugins() -> Plugins {
    Plugins(HashMap::from([
        ("sm".to_owned(), Plugin { prefix: "sm".to_owned(), exec: "/home/davawen/.config/keal/plugins/session_manager/exec.sh".to_owned(), comment: None }),
        ("em".to_owned(), Plugin { prefix: "em".to_owned(), exec: "/home/davawen/.config/keal/plugins/emoji/exec".to_owned(), comment: Some("Get emojis from their name".to_owned()) })
    ]))
}

#[derive(Debug)]
pub struct PluginEntry {
    prefix: String,
    comment: Option<String>
}

impl EntryTrait for PluginEntry {
    fn name(&self) ->  &str { &self.prefix }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn to_match(&self) ->  &str { &self.prefix }
}

pub fn plugin_entries(plugins: &Plugins) -> impl Iterator<Item = PluginEntry> + '_ {
    plugins.0.values()
        .map(|plugin| PluginEntry { prefix: plugin.prefix.clone(), comment: plugin.comment.clone() })
}

#[derive(Debug)]
pub struct FieldEntry {
    field: String,
    icon: Option<String>,
    comment: Option<String>
}

impl EntryTrait for FieldEntry {
    fn name(&self) ->  &str { &self.field }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn to_match(&self) ->  &str { &self.field }
}
