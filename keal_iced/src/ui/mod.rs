use std::os::unix::process::CommandExt;

use fork::{fork, Fork};
use iced::{futures::channel::mpsc, keyboard::{self, key::{Key, Named}, Modifiers}, widget::{button, column as icolumn, container, image, row as irow, scrollable, svg, text, text_input, Space}, Element, Length, Padding, Subscription, Task};
use nucleo_matcher::Matcher;

use keal::{icon::{IconCache, Icon}, config::config, plugin::{Action, entry::{Label, OwnedEntry}}, log_time};

pub use crate::config::Theme;
use styled::{ButtonStyle, TextStyle};

use self::{match_span::MatchSpan, async_manager::AsyncManager};

mod styled;
mod match_span;
mod async_manager;

pub struct Keal {
    // Global state
    theme: Theme,

    // UI state
    input: String,
    selected: usize,

    // data state
    icons: IconCache,

    entries: Vec<OwnedEntry>,
    manager: AsyncManager,
    sender: Option<mpsc::Sender<async_manager::Event>>,

    first_event: bool
}

#[derive(Debug, Clone)]
pub enum Message {
    // UI events
    TextInput(String),
    Launch(Option<Label>),
    KeyPress(Key, Modifiers),

    // Worker events
    IconCacheLoaded(IconCache),
    SenderLoaded(mpsc::Sender<async_manager::Event>),
    Entries(Vec<OwnedEntry>),
    Action(Action),
}

fn close_main_window() -> Task<Message> {
    iced::window::get_oldest().and_then(|id| {
        iced::window::close(id)
    })
}

impl Keal {
    pub fn theme(&self) -> Theme {
        self.theme.clone()
    }

    pub fn new(theme: Theme) -> (Self, Task<Message>) {
        log_time("initializing app");

        let config = config();

        let focus = text_input::focus(text_input::Id::new("query_input")); // focus input on start up

        let icon_theme = config.icon_theme.clone();
        let load_icons = Task::perform(async move {
            IconCache::new(&icon_theme)
        }, Message::IconCacheLoaded);

        let command = Task::batch(vec![focus, load_icons]);
        let manager = AsyncManager::new(Matcher::default(), 50, true);

        log_time("finished initializing");

        (Keal {
            theme,
            input: String::new(),
            selected: 0,
            icons: IconCache::default(),
            entries: Vec::new(),
            manager,
            sender: None,
            first_event: false
        }, command)
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let key_press = keyboard::on_key_press(|key, mods| {
            Some(Message::KeyPress(key, mods))
        });

        let manager = Subscription::run_with_id("manager", self.manager.subscription());
        Subscription::batch([key_press, manager])
    }

    pub fn view(&self) -> iced::Element<'_, Message, Theme> {
        let entries = &self.entries;
        let config = config();

        let input = text_input(&config.placeholder_text, &self.input)
            .on_input(Message::TextInput)
            .on_submit(Message::Launch(entries.get(self.selected).map(|e| e.label)))
            .size(config.font_size * 1.25).padding(config.font_size)
            .id(text_input::Id::new("query_input"));

        let input = container(input)
            .width(Length::Fill);

        let data = &mut *self.manager.get_data();
        let mut buf = vec![];

        let entries = scrollable(icolumn({
            entries.iter().enumerate().map(|(index, entry)| {
                let selected = self.selected == index;

                let mut item = irow(vec![]);

                if let Some(icon) = &entry.icon {
                    if let Some(icon) = self.icons.get(icon) {
                        let element: Element<_, _> = match icon {
                            Icon::Svg(path) => svg(svg::Handle::from_path(path)).width(config.font_size).height(config.font_size).into(),
                            Icon::Other(path) => image(path).width(config.font_size).height(config.font_size).into()
                        };
                        item = item.push(container(element).padding(4));
                    }
                }

                for (span, highlighted) in MatchSpan::new(&entry.name, &mut data.matcher, &data.pattern, &mut buf) {
                    item = item.push(text(span).size(config.font_size).shaping(self.theme.text_shaping).class(
                        match highlighted {
                            false => TextStyle::Normal,
                            true => TextStyle::Matched { selected },
                        }
                    ));
                }

                item = item.push(Space::with_width(Length::Fill)); // fill the whole line up
                if let Some(comment) = &entry.comment {
                    item = item.push(Space::with_width(5.0)); // minimum amount of space between name and comment
                    item = item.push(
                        text(comment)
                            .size(config.font_size)
                            .shaping(self.theme.text_shaping)
                            .class(TextStyle::Comment)
                    );
                }

                button(item)
                    .on_press(Message::Launch(Some(entry.label)))
                    .class(if selected { ButtonStyle::Selected } else { ButtonStyle::Normal })
                    .padding(Padding { right: 20.0, ..Padding::new(10.0) })
            })
                .map(Element::<_, _>::from)
        })).id(scrollable::Id::new("scrollable"));

        icolumn![ input, entries ]
            .width(Length::Fill).height(Length::Fill)
            .into()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        if !self.first_event {
            self.first_event = true;
            log_time("recieved first event");
        }

        // iced::window::fetch_size(f)
        // scrollable::Properties::default().width

        match message {
            Message::KeyPress(key, mods) => match (key.as_ref(), mods) {
                (Key::Named(Named::Escape), _) => return close_main_window(),
                // TODO: gently scroll window to selected choice
                (Key::Character("j" | "n"), Modifiers::CTRL)  | (Key::Named(Named::ArrowDown), _)  => {
                    self.selected += 1;
                    self.selected = self.selected.min(self.entries.len().saturating_sub(1));
                }
                (Key::Character("k" | "p"), Modifiers::CTRL) | (Key::Named(Named::ArrowUp), _) => {
                    self.selected = self.selected.saturating_sub(1);
                }
                _ => ()
            }
            Message::TextInput(input) => self.update_input(input, true),
            Message::Launch(selected) => {
                if let Some(sender) = &mut self.sender {
                    sender.try_send(async_manager::Event::Launch(selected)).expect("failed to send launch command");
                }
            }
            Message::IconCacheLoaded(icon_cache) => self.icons = icon_cache,
            Message::Entries(entries) => self.entries = entries,
            Message::SenderLoaded(sender) => {
                self.sender = Some(sender);
                self.update_input(self.input.clone(), true); // in case the user typed in before the manager was loaded
            },
            Message::Action(action) => return self.handle_action(action),
        };

        Task::none()
    }
}

impl Keal {
    pub fn update_input(&mut self, input: String, from_user: bool) {
        self.input = input.clone();
        if let Some(sender) = &mut self.sender {
            sender.try_send(async_manager::Event::UpdateInput(input, from_user)).expect("failed to send update input command");
        }
    }

    fn handle_action(&mut self, action: Action) -> Task<Message> {
        match action {
            Action::None => (),
            Action::ChangeInput(new) => {
                self.manager.with_manager(|m| m.kill());
                self.update_input(new, false);
                return text_input::move_cursor_to_end(text_input::Id::new("query_input"));
            }
            Action::ChangeQuery(new) => {
                let new = self.manager.use_manager(|m| m.current().map(
                    |plugin| format!("{} {}", plugin.prefix, new) 
                )).unwrap_or(new);
                self.update_input(new, false);

                return text_input::move_cursor_to_end(text_input::Id::new("query_input"));
            }
            Action::Exec(mut command) => {
                let _ = command.0.exec();
                return close_main_window();
            }
            Action::PrintAndClose(message) => {
                println!("{message}");
                return close_main_window();
            }
            Action::Fork => match fork().expect("failed to fork") {
                Fork::Parent(_) => return close_main_window(),
                Fork::Child => ()
            }
            Action::WaitAndClose => {
                self.manager.with_manager(|m| m.wait());
                return close_main_window();
            }
        }

        Task::none()
    }
}
