use std::{iter::Peekable, process::{ChildStdin, ChildStdout}, io::{BufReader, Lines, BufRead, Write}, path::{Path, PathBuf}, fs};

use bitflags::bitflags;
use nucleo_matcher::{Matcher, pattern::Pattern};

use crate::{ini_parser::Ini, icon::IconPath, config::Config, xdg_utils::config_dir, plugin::{PluginExecution, Plugin, Entry, Action}};

/// returns `None` if the plugin directory does not exist
pub fn get_user_plugins() -> Option<impl Iterator<Item = (String, Plugin)>> {
    let mut config = config_dir().ok()?;
    config.push("plugins");

    let plugins = fs::read_dir(config).ok()?;

    Some(plugins
        .flatten()
        .filter(|entry| entry.file_type().unwrap().is_dir())
        .map(|entry| entry.path())
        .map(|path| (path.join("config.ini"), path))
        .flat_map(|(config, path)| Some((Ini::from_file(config, &['#', ';']).ok()?, path)))
        .flat_map(|(config, path)| UserPlugin::create(&path, config))
        .map(|plugin| (plugin.prefix.clone(), plugin)))
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

struct PluginEntry {
    name: String,
    comment: Option<String>,
    icon: Option<IconPath>
}


// TODO: Better error handling for plugins: instead of panicking or logging to stderr, show feedback in window
// TODO: Asynchronous/Non blocking plugins

pub struct UserPlugin {
    entries: Vec<PluginEntry>,
    child: std::process::Child,
    stdin: ChildStdin,
    stdout: Peekable<Lines<BufReader<ChildStdout>>>,
    events: PluginEvents,
    cwd: PathBuf
}

impl UserPlugin {
    /// creates a `Plugin` with a `UserPlugin` generator
    fn create(plugin_path: &Path, mut ini: Ini) -> Option<Plugin> {
        let config = ini.remove_section("config").map(|c| c.into_map()).unwrap_or_default();
        let mut ini = ini.remove_section("plugin")?.into_map();

        let exec = plugin_path.join(ini.swap_remove("exec")?);
        Some(Plugin {
            name: ini.swap_remove("name")?,
            icon: ini.swap_remove("icon").map(|i| IconPath::new(i, Some(plugin_path))),
            comment: ini.swap_remove("comment"),
            prefix: ini.swap_remove("prefix")?,
            config,
            generator: Box::new(move |plugin, _| {
                use std::process::{Stdio, Command};

                let cwd = exec.parent().unwrap().to_path_buf();
                let mut child = Command::new(&exec)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .current_dir(&cwd)
                    .spawn().expect("Couldn't spawn process from plugin");

                let stdin = child.stdin.take().unwrap();
                let stdout = child.stdout.take().unwrap();
                let stdout = BufReader::new(stdout).lines().peekable();

                let mut this = Self {
                    entries: vec![],
                    child, stdin, stdout, events: PluginEvents::None, cwd
                };

                this.send_config(plugin);
                this.get_events();
                this.entries = this.get_choice_list();
                Box::new(this)
            })
        })
    }

    fn send_config(&mut self, plugin: &Plugin) {
        for config in plugin.config.values() {
            writeln!(self.stdin, "{config}").unwrap();
        }
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

    fn get_action(&mut self) -> Action {
        let line = self.stdout.next().unwrap().unwrap();

        match line.split_once(':') {
            Some(("action", action)) => match action.split_once(':') {
                Some(("change_input", value)) => Action::ChangeInput(value.to_owned()),
                Some(("change_query", value)) => Action::ChangeQuery(value.to_owned()),
                Some(("update", index)) => {
                    let index: usize = index.parse().unwrap();
                    let element = self.get_choice_list().pop().expect("one element for update action");
                    self.entries[index] = element;
                    Action::None
                }
                _ => match action {
                    "fork" => Action::Fork,
                    "wait_and_close" => Action::WaitAndClose,
                    "update_all" => {
                        self.entries = self.get_choice_list();
                        Action::None
                    },
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
            entries.push(PluginEntry { name, icon, comment });
        }

        entries
    }
}

impl Drop for UserPlugin {
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => (), // process has already exited
            _ => {
                let _ = self.child.kill(); // ignore any resulting error
            }
        }
    }
}

impl PluginExecution for UserPlugin {
    fn finished(&mut self) -> bool {
        self.child.try_wait().unwrap().is_some()
    }

    fn wait(&mut self) {
        let _ = self.child.wait();
    }
    
    fn send_query(&mut self, _: &Config, query: &str) -> Action {
        if !self.events.intersects(PluginEvents::Query) { return Action::None }

        writeln!(self.stdin, "query\n{query}").unwrap();
        self.get_action()
    }

    fn send_enter(&mut self, _: &Config, _: &str, idx: Option<usize>) -> Action {
        if !self.events.intersects(PluginEvents::Enter) { return Action::None }
        let Some(idx) = idx else { return Action::None };

        writeln!(self.stdin, "enter\n{idx}").unwrap();
        self.get_action()
    }

    fn get_entries<'a>(&'a self, _: &Config, matcher: &mut Matcher, pattern: &Pattern, out: &mut Vec<Entry<'a>>) {
        let mut charbuf = vec![];
        for (index, entry) in self.entries.iter().enumerate() {
            let Some(entry) = Entry::new(matcher, pattern, &mut charbuf, &entry.name, entry.icon.as_ref(), entry.comment.as_deref(), index)
                else { continue };

            out.push(entry);
        }
    }

    fn get_name(&self, index: usize) -> &str {
        &self.entries[index].name
    }
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
