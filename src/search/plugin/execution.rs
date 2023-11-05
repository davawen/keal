use std::{process::{ChildStdin, ChildStdout}, io::{BufRead, BufReader}};

use crate::search::EntryTrait;

use super::Plugin;

/// The execution of the currently typed plugin, used for RPC.
/// The underlying process will get killed when it is dropped, so you don't have to fear zombie processes.
#[derive(Debug)]
pub struct PluginExecution {
    pub prefix: String,
    pub child: std::process::Child,
    pub stdin: ChildStdin,
    pub stdout: std::io::Lines<BufReader<ChildStdout>>,
    pub entries: Vec<crate::search::Entry>
}

impl PluginExecution {
    pub fn new(plugin: &Plugin) -> Self {
        use std::process::{Stdio, Command};

        let mut child = Command::new(&plugin.exec)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .current_dir(plugin.exec.parent().unwrap())
            .spawn().expect("Couldn't spawn process from plugin");

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut stdout = BufReader::new(stdout).lines();

        let mut entries = vec![];
        let mut current: Option<FieldEntry> = None;

        // Read initial entries line by line
        for line in stdout.by_ref() {
            let Ok(line) = line else { continue };

            if let Some(line) = line.strip_prefix("name:") {
                if let Some(old) = current.take() {
                    entries.push(old.into());
                }

                current = Some(FieldEntry {
                    field: line.to_owned(),
                    icon: None,
                    comment: None
                });
            } else if let (Some(current), Some(line)) = (&mut current, line.strip_prefix("icon:")) {
                current.icon = Some(line.to_owned());
            } else if let (Some(current), Some(line)) = (&mut current, line.strip_prefix("comment:")) {
                current.comment = Some(line.to_owned());
            } else if line == "end" {
                if let Some(old) = current.take() {
                    entries.push(old.into());
                }
                break
            } else if line.starts_with("icon:") || line.starts_with("comment:") {
                eprintln!("using a modifier descriptor before setting a field in plugin `{}`", plugin.prefix);
            } else {
                eprintln!("unknown descriptor `{line}` in plugin `{}`", plugin.prefix)
            }
        }

        Self {
            prefix: plugin.prefix.clone(),
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

#[derive(Debug)]
pub struct FieldEntry {
    field: String,
    comment: Option<String>,
    icon: Option<String>
}

impl EntryTrait for FieldEntry {
    fn name(&self) ->  &str { &self.field }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn icon(&self) -> Option<&str> { self.icon.as_deref() }
    fn to_match(&self) ->  &str { &self.field }
}
