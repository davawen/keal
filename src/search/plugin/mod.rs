use std::{collections::HashMap, fs, path::{Path, PathBuf}};

use tini::Ini;

pub mod execution;
use execution::PluginExecution;

use crate::icon::IconPath;

use super::EntryTrait;

#[derive(Debug, Clone)]
pub struct Plugin {
    pub name: String,
    pub icon: Option<IconPath>,
    pub comment: Option<String>,
    pub prefix: String,
    pub exec: PathBuf,
}

impl Plugin {
    pub fn new(plugin_path: &Path, ini: Ini) -> Option<Self> {
        let mut ini: HashMap<_, _> = ini.section_iter("plugin")
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect();

        Some(Self {
            name: ini.remove("name")?,
            icon: ini.remove("icon").map(IconPath::from),
            comment: ini.remove("comment"),
            prefix: ini.remove("prefix")?,
            exec: plugin_path.join(ini.remove("exec")?)
        })
    }

    pub fn generate(&self) -> PluginExecution {
        PluginExecution::new(self)
    }
}

#[derive(Debug)]
pub struct PluginEntry {
    prefix: String,
    comment: String,
    icon: Option<IconPath>
}

impl EntryTrait for PluginEntry {
    fn name(&self) ->  &str { &self.prefix }
    fn comment(&self) -> Option<&str> { Some(&self.comment) }
    fn icon(&self) -> Option<&IconPath> { self.icon.as_ref() }
    fn to_match(&self) ->  &str { &self.prefix }
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

pub fn plugin_entries(plugins: &Plugins) -> impl Iterator<Item = PluginEntry> + '_ {
    plugins.0.values()
        .map(|plugin| PluginEntry {
            prefix: plugin.prefix.clone(),
            icon: plugin.icon.clone(),
            comment: format!("{}{}", plugin.name, plugin.comment.as_ref()
                .map(|s| format!(" ({s})"))
                .unwrap_or_default()
            )
        })
}
