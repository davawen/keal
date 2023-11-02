use std::{collections::HashMap, io::{BufRead, BufReader}, process::{ChildStdout, ChildStdin}, fs, str::FromStr, path::{Path, PathBuf}};

use tini::Ini;

use super::EntryTrait;

#[derive(Debug, Clone)]
pub struct Plugin {
    pub prefix: String,
    pub comment: Option<String>,
    pub icon: Option<String>,
    pub exec: PathBuf,
    pub kind: PluginKind
}

#[derive(Debug, Clone)]
pub enum PluginKind {
    Text, Json
}

impl FromStr for PluginKind {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            _ => Err(())
        }
    }
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
    pub fn new(plugin_path: &Path, ini: Ini) -> Option<Self> {
        let mut ini: HashMap<_, _> = ini.section_iter("plugin")
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect();

        Some(Self {
            prefix: ini.remove("prefix")?,
            comment: ini.remove("comment"),
            icon: ini.remove("icon"),
            exec: plugin_path.join(ini.remove("exec")?),
            kind: ini.remove("type")?.parse().ok()?
        })
    }

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

            match (&mut current, line.get(0..1)) {
                (old, Some("-")) => {
                    if let Some(old) = old.take() {
                        entries.push(old.into());
                    }
                    current = Some(FieldEntry {
                        field: line[1..].to_owned(),
                        icon: None,
                        comment: None
                    });
                }
                (Some(ref mut current), Some("*")) => current.icon = Some(line[1..].to_owned()),
                (Some(ref mut current), Some("=")) => current.comment = Some(line[1..].to_owned()),
                (current, Some("%")) => {
                    if let Some(current) = current.take() { entries.push(current.into()) }
                    break
                }
                (None, Some("*" | "=")) => eprintln!("using a modifier descriptor before setting a field in plugin `{}`", self.prefix),
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

#[derive(Debug, Default)]
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
    let Ok(config) = std::env::var("XDG_CONFIG_HOME") else {
        eprintln!("$XDG_CONFIG_HOME is not configured. Didn't load any plugin.");
        return Plugins::default()
    };

    let path = format!("{config}/keal/plugins");
    let Ok(plugins) = fs::read_dir(path) else { return Plugins::default() };

    Plugins(plugins
        .flatten()
        .filter(|entry| entry.file_type().unwrap().is_dir())
        .map(|entry| entry.path())
        .map(|path| (path.join("config.ini"), path))
        .flat_map(|(config, path)| Ok::<_, tini::Error>((Ini::from_file(&config)?, path)))
        .flat_map(|(config, path)| Plugin::new(&path, config))
        .map(|plugin| (plugin.prefix.clone(), plugin))
        .collect())
}

#[derive(Debug)]
pub struct PluginEntry {
    prefix: String,
    comment: Option<String>,
    icon: Option<String>
}

impl EntryTrait for PluginEntry {
    fn name(&self) ->  &str { &self.prefix }
    fn comment(&self) -> Option<&str> { self.comment.as_deref() }
    fn icon(&self) -> Option<&str> { self.icon.as_deref() }
    fn to_match(&self) ->  &str { &self.prefix }
}

pub fn plugin_entries(plugins: &Plugins) -> impl Iterator<Item = PluginEntry> + '_ {
    plugins.0.values()
        .map(|plugin| PluginEntry { prefix: plugin.prefix.clone(), comment: plugin.comment.clone(), icon: plugin.icon.clone() })
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
