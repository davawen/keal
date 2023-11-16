use std::{process, os::unix::process::CommandExt};

use fork::{fork, Fork};
use fuzzy_matcher::skim::SkimMatcherV2;
use iced::{Application, executor, Command, widget::{row as irow, text_input, column as icolumn, container, text, Space, scrollable, button, image, svg}, font, Element, Length, subscription, Event, keyboard::{self, KeyCode, Modifiers}};

use crate::{search::{self, plugin::{get_plugins, execution::{PluginExecution, PluginAction}, Plugin}, create_entries, EntryTrait, Entry}, icon::{IconCache, Icon}, config::Config};

pub use styled::Theme;
use styled::{ButtonStyle, TextStyle};

mod styled;

type Matcher = SkimMatcherV2;

pub struct Keal {
    input: String,
    query: String,
    matcher: Matcher,
    plugins: search::plugin::Plugins,
    entries: Vec<search::Entry>,
    shown: Shown,
    selected: usize,
    icons: IconCache,
    config: Config
}

enum Shown {
    Entries(Vec<(usize, i64)>),
    Plugin {
        execution: PluginExecution,
        filtered: Vec<(usize, i64)>
    }
}

impl Shown {
    fn filtered(&self) -> &[(usize, i64)] {
        match self {
            Shown::Entries(f) => f,
            Shown::Plugin { filtered: f, .. } => f
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    FontLoaded(Result<(), font::Error>),
    TextInput(String),
    Launch(usize),
    Event(keyboard::Event)
}

impl Application for Keal {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = Config;

    fn new(config: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let plugins = get_plugins();
        let entries = create_entries(&plugins);
        // show the 50 first elements in the beginning
        // TODO: fuzzel-like often launched applications
        let filtered = entries.iter().take(50).enumerate().map(|(i, _)| (i, 0)).collect();
        let icons = IconCache::new(&config.icon_theme);

        let this = Keal {
            input: String::new(), query: String::new(),
            matcher: Matcher::default(),
            plugins, entries,
            shown: Shown::Entries(filtered),
            selected: 0,
            icons,
            config
        };

        let focus = text_input::focus(text_input::Id::new("query_input")); // focus input on start up

        let iosevka = include_bytes!("../../public/iosevka-regular.ttf");
        let iosevka = font::load(iosevka.as_slice()).map(Message::FontLoaded);

        (this, Command::batch(vec![iosevka, focus]))
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        subscription::events_with(|event, _status| match event {
            Event::Keyboard(k) => Some(Message::Event(k)),
            _ => None
        })
    }

    fn title(&self) -> String {
        "Keal".to_owned()
    }

    fn theme(&self) -> Self::Theme {
        self.config.theme.clone() // unfortunate clone, not sure how to get rid of this
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        let input = text_input(&self.config.placeholder_text, &self.input)
            .on_input(Message::TextInput)
            .on_submit(Message::Launch(self.selected))
            .size(self.config.font_size * 1.25).padding(self.config.font_size)
            .id(text_input::Id::new("query_input"));

        let input = container(input)
            .width(Length::Fill);

        let entries = scrollable(icolumn({
            let (entries, filtered) = match &self.shown {
                Shown::Entries(filtered) => (&self.entries, filtered),
                Shown::Plugin { execution, filtered } => (&execution.entries, filtered)
            };

            filtered.iter().enumerate().map(|(index, &(entry, _))| {
                let entry = &entries[entry];
                let selected = self.selected == index;

                let mut item = irow(vec![]);

                if let Some(icon) = entry.icon() {
                    if let Some(icon) = self.icons.get(icon) {
                        let element: Element<_, _> = match icon {
                            Icon::Svg(path) => svg(svg::Handle::from_path(path)).width(self.config.font_size).height(self.config.font_size).into(),
                            Icon::Other(path) => image(path).width(self.config.font_size).height(self.config.font_size).into()
                        };
                        item = item.push(container(element).padding(4));
                    }
                }

                for (span, highlighted) in entry.fuzzy_match_span(&self.matcher, &self.query) {
                    item = item.push(text(span).size(self.config.font_size).style(
                        match highlighted {
                            false => TextStyle::Normal,
                            true => TextStyle::Matched { selected },
                        }
                    ));
                }
                item = item.push(Space::with_width(Length::Fill));
                item = item.push(
                    text(entry.comment().unwrap_or(""))
                        .size(self.config.font_size)
                        .style(TextStyle::Comment)
                );

                button(item)
                    .on_press(Message::Launch(index))
                    .style(if selected { ButtonStyle::Selected } else { ButtonStyle::Normal })
                    .padding([10, 20, 10, 10])
            })
                .map(Element::<_, _>::from)
                .collect()
        }));

        icolumn![ input, entries ]
            .width(Length::Fill).height(Length::Fill)
            .into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        use keyboard::Event::KeyPressed;
        match message {
            Message::FontLoaded(_) => (),
            Message::TextInput(input) => return self.update_input(input, true),
            Message::Event(event) => match event {
                KeyPressed { key_code: KeyCode::Escape, .. } => return iced::window::close(),
                // TODO: gently scroll window to selected choice
                KeyPressed { key_code: KeyCode::J, modifiers: Modifiers::CTRL } | KeyPressed { key_code: KeyCode::Down, .. } => {
                    self.selected += 1;
                    self.selected = self.selected.min(self.shown.filtered().len().saturating_sub(1));
                }
                KeyPressed { key_code: KeyCode::K, modifiers: Modifiers::CTRL }
                | KeyPressed { key_code: KeyCode::Up, .. } => {
                    self.selected = self.selected.saturating_sub(1);
                }
                _ => ()
            }
            Message::Launch(selected) => match &mut self.shown {
                Shown::Entries(filtered) => match &self.entries[filtered[selected].0] {
                    // complete plugin prefix
                    Entry::PluginEntry(plugin) => {
                        let input = format!("{} ", plugin.name());
                        let c = self.update_input(input, true);
                        return Command::batch([c, text_input::move_cursor_to_end(text_input::Id::new("query_input"))]);
                    }
                    // launch application and close window
                    Entry::DesktopEntry(app) => {
                         // TODO: parse XDG desktop parameters
                        process::Command::new("sh") // ugly work around to avoir parsing spaces/quotes
                            .arg("-c")
                            .arg(&app.exec)
                            .exec();

                        return iced::window::close()
                    }
                    _ => unreachable!("something went terribly wrong")
                }
                // send selected action to plugin
                Shown::Plugin { execution, filtered } => if let Some(&(selected, _)) = filtered.get(selected) {
                    if let Entry::FieldEntry(_) = &execution.entries[selected] {
                        if let Some(action) = execution.send_enter(selected) {
                            return self.handle_action(action);
                        }
                    } else {
                        eprintln!("something went terribly wrong: non field entry in plugin entries");
                    }
                }
            }
        };

        Command::none()
    }
}

impl Shown {
    fn launch(&mut self, plugin: &Plugin) {
        *self = Shown::Plugin { 
            execution: plugin.generate(),
            filtered: Vec::new() // filter happens right after
        };
    }
}

impl Keal {

    /// Changes the input field to a new value
    /// `from_user` describes wether this change originates from user interaction
    /// Or wether it comes from a plugin action, (and should therefore not be propagated as an event, to avoid cycles)
    fn update_input(&mut self, input: String, from_user: bool) -> Command<Message> {
        self.input = input;

        let mut command = Command::none();

        // launch or stop plugin execution depending on new state of filter
        // if in plugin mode, remove plugin prefix from filter
        self.query = match (self.plugins.filter_starts_with_plugin(&self.input), &mut self.shown) {
            (Some((plugin, remainder)), Shown::Entries(_)) => { // launch plugin
                self.shown.launch(plugin);
                remainder.to_owned()
            }
            (Some((plugin, remainder)), Shown::Plugin { execution, .. }) => {
                // relaunch plugin if it is done executing or if we're currently executing the wrong plugin
                let remainder = remainder.to_owned();
                if execution.child.try_wait().unwrap().is_some() || plugin.prefix != execution.prefix {
                    self.shown.launch(plugin);
                } else if from_user { // send query event
                    if let Some(action) = execution.send_query(&remainder) {
                        command = self.handle_action(action);
                    };
                }
                remainder
            }
            (None, Shown::Plugin { .. }) => { // stop plugin
                self.shown = Shown::Entries(Vec::new());
                self.input.clone()
            }
            (None, Shown::Entries(_)) => self.input.clone()
        };

        self.filter();
        command
    }

    fn filter(&mut self) {
        match &mut self.shown {
            Shown::Entries(filtered) => *filtered = search::filter_entries(&self.matcher, &self.entries, &self.query, 50),
            Shown::Plugin { execution, filtered } =>
                *filtered = search::filter_entries(&self.matcher, &execution.entries, &self.query, 50)
        }
    }

    /// panics if `self.shown` is not Shown::Plugin
    fn handle_action(&mut self, action: PluginAction) -> Command<Message> {
        let Shown::Plugin { execution, .. } = &mut self.shown else {
            panic!("Trying to handle action on plugin that isn't loaded")
        };

        use PluginAction as Action;

        match action {
            Action::Fork => match fork().expect("failed to fork") {
                Fork::Parent(_) => return iced::window::close(),
                Fork::Child => ()
            }
            Action::WaitAndClose => {
                let _ = execution.child.wait();
                return iced::window::close();
            }
            Action::ChangeInput(new) => {
                self.shown = Shown::Entries(Vec::new()); // kill running plugin
                let c = self.update_input(new, false);
                return Command::batch([c, text_input::move_cursor_to_end(text_input::Id::new("query_input"))]);
            }
            Action::ChangeQuery(new) => {
                let new = format!("{} {}", execution.prefix, new);
                let c = self.update_input(new, false);
                return Command::batch([c, text_input::move_cursor_to_end(text_input::Id::new("query_input"))]);
            }
            Action::Update(idx, entry) => {
                execution.entries[idx] = entry;
                self.filter();
            }
            Action::UpdateAll(entries) => {
                execution.entries = entries;
                self.filter();
            }
            Action::None => ()
        }

        Command::none()
    }
}
