use crate::{icon::IconPath, plugin::{Plugin, PluginExecution, Action, entry::Entry}, config::Config};

struct ListEntry {
    name: String,
    icon: Option<IconPath>,
    comment: Option<String>
}

pub struct ListPlugin(Vec<ListEntry>);

impl ListPlugin {
    /// this should be added LAST to list every existing plugin
    pub fn create() -> Plugin {
        Plugin {
            name: "List".to_owned(),
            prefix: "ls".to_owned(),
            icon: None,
            comment: Some("List loaded keal plugins".to_owned()),
            generator: Box::new(|_, manager| {
                let entries = manager.list_plugins()
                    .map(|(prefix, plug)| ListEntry {
                        name: prefix.clone(),
                        icon: plug.icon.clone(),
                        comment: Some(plug.comment.as_ref()
                            .map(|c| format!("{} ({c})", plug.name))
                            .unwrap_or(plug.name.clone()))
                    })
                    .collect();

                Box::new(ListPlugin(entries))
            })
        }
    }
}

impl PluginExecution for ListPlugin {
    fn finished(&mut self) -> bool { false }
    fn wait(&mut self) { }
    fn send_query(&mut self, _: &Config, _: &str) -> Action { Action::None }

    fn send_enter(&mut self, _: &Config, _: &str, idx: Option<usize>) -> Action {
        if let Some(idx) = idx {
            let prefix = self.0[idx].name.clone();
            Action::ChangeInput(format!("{prefix} "))
        } else {
            Action::None
        }
    }

    fn get_entries<'a>(&'a self, _: &Config, matcher: &mut nucleo_matcher::Matcher, pattern: &nucleo_matcher::pattern::Pattern, out: &mut Vec<crate::plugin::entry::Entry<'a>>) {
        let mut charbuf = vec![];
        for (index, entry) in self.0.iter().enumerate() {
            let Some(entry) = Entry::new(matcher, pattern, &mut charbuf, &entry.name, entry.icon.as_ref(), entry.comment.as_deref(), index) else { continue };

            out.push(entry);
        }
    }

    fn get_name(&self, index: usize) -> &str {
        &self.0[index].name
    }
}
