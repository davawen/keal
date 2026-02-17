use std::process::Command;

use nucleo_matcher::{pattern::Pattern, Matcher};

use crate::{
    config::Config,
    icon::IconPath,
    plugin::{entry::Entry, Action, Plugin, PluginExecution},
};

struct SessionEntry {
    name: String,
    icon: Option<IconPath>,
    command: String,
}

pub struct SessionPlugin(Vec<SessionEntry>);

impl SessionPlugin {
    pub fn create() -> Plugin {
        let log_out = if let Ok(env) = std::env::var("XDG_CURRENT_DESKTOP") {
            match env.as_str() {
                "Unity" | "Pantheon" | "GNOME" => "gnome-session-quit --logout".to_owned(),
                "niri" | "Niri" => "niri msg action quit".to_owned(),
                "sway" | "Sway" => "swaymsg exit".to_owned(),
                "kde-plasma" | "KDE" => {
                    "qdbus org.kde.ksmserver /KSMServer logout 0 0 0".to_owned()
                }
                "X-Cinnamon" | "Cinnamon" => "cinnamon-session-quit --logout".to_owned(),
                "MATE" => "mate-session-save --logout-dialog".to_owned(),
                "XFCE" => "xfce4-session-logout --logout".to_owned(),
                _ => {
                    if std::env::var_os("SWAYSOCK").is_some() {
                        "swaymsg exit".to_owned()
                    } else {
                        eprintln!("session manager: failted to auto-detect environment");
                        String::new()
                    }
                }
            }
        } else {
            String::new()
        };

        let config = indexmap::IndexMap::from([
            ("log_out".to_owned(), log_out),
            ("suspend".to_owned(), "systemctl suspend".to_owned()),
            ("hibernate".to_owned(), "systemctl hibernate".to_owned()),
            ("reboot".to_owned(), "systemctl reboot".to_owned()),
            ("poweroff".to_owned(), "systemctl poweroff".to_owned()),
        ]);

        Plugin {
            name: "Session Manager".to_owned(),
            prefix: "sm".to_owned(),
            icon: None,
            config,
            comment: Some("Manage current session".to_owned()),
            generator: Box::new(move |plugin, _| {
                let mut entries = Vec::new();
                let mut add = |name: &str, id: &str| {
                    if !plugin.config[id].is_empty() {
                        entries.push(SessionEntry {
                            name: name.to_owned(),
                            command: plugin.config[id].to_owned(),
                            icon: None,
                        });
                    }
                };

                add("Log Out", "log_out");
                add("Suspend", "suspend");
                add("Hibernate", "hibernate");
                add("Reboot", "reboot");
                add("Power off", "poweroff");

                Box::new(SessionPlugin(entries))
            }),
        }
    }
}

impl PluginExecution for SessionPlugin {
    fn finished(&mut self) -> bool {
        false
    }
    fn wait(&mut self) {}

    fn send_query(&mut self, _: &Config, _: &str) -> Action {
        Action::None
    }
    fn send_enter(&mut self, _: &Config, _: &str, idx: Option<usize>) -> Action {
        let Some(idx) = idx else { return Action::None };

        let mut command = Command::new("sh");
        command.arg("-c").arg(&self.0[idx].command);

        Action::Exec(command.into())
    }

    fn get_entries<'a>(
        &'a self,
        _: &Config,
        matcher: &mut Matcher,
        pattern: &Pattern,
        out: &mut Vec<Entry<'a>>,
    ) {
        let mut charbuf = vec![];
        for (index, entry) in self.0.iter().enumerate() {
            let Some(entry) = Entry::new(
                matcher,
                pattern,
                &mut charbuf,
                &entry.name,
                entry.icon.as_ref(),
                None,
                index,
            ) else {
                continue;
            };

            out.push(entry);
        }
    }

    fn get_name(&self, index: usize) -> &str {
        &self.0[index].name
    }
}
