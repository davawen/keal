use std::os::unix::process::CommandExt;

use fork::{fork, Fork};
use iced::{Application, executor, Command, widget::{row as irow, text_input, column as icolumn, container, text, Space, scrollable, button, image, svg}, font, Element, Length, subscription, Event, keyboard::{self, KeyCode, Modifiers}, futures::channel::mpsc};
use nucleo_matcher::Matcher;

use keal::{icon::{IconCache, Icon}, config::config, plugin::{Action, entry::{Label, OwnedEntry}}, log_time};

pub use crate::config::Theme;
use styled::{ButtonStyle, TextStyle};

use self::{match_span::MatchSpan, async_manager::AsyncManager};

mod styled;
mod match_span;
mod async_manager;

pub struct Keal {
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
    Event(keyboard::Event),

    // Worker events
    FontLoaded(Result<(), font::Error>),
    IconCacheLoaded(IconCache),
    SenderLoaded(mpsc::Sender<async_manager::Event>),
    Entries(Vec<OwnedEntry>),
    Action(Action)
}

impl Application for Keal {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = Theme;

    fn new(theme: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        log_time("initializing app");

        let config = config();

        let focus = text_input::focus(text_input::Id::new("query_input")); // focus input on start up

        let iosevka = include_bytes!("../../../public/iosevka-regular.ttf");
        let iosevka = font::load(iosevka.as_slice()).map(Message::FontLoaded);

        let icon_theme = config.icon_theme.clone();
        let load_icons = Command::perform(async move {
            IconCache::new(&icon_theme)
        }, Message::IconCacheLoaded);

        let command = Command::batch(vec![iosevka, focus, load_icons]);
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

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let events = subscription::events_with(|event, _status| match event {
            Event::Keyboard(k) => Some(Message::Event(k)),
            _ => None
        });

        let manager = self.manager.subscription();
        subscription::Subscription::batch([events, manager])
    }

    fn title(&self) -> String {
        "Keal".to_owned()
    }

    fn theme(&self) -> Self::Theme {
        self.theme.clone() // unfortunate clone, not sure how to get rid of this
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
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
                    item = item.push(text(span).size(config.font_size).shaping(self.theme.text_shaping).style(
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
                            .style(TextStyle::Comment)
                    );
                }

                button(item)
                    .on_press(Message::Launch(Some(entry.label)))
                    .style(if selected { ButtonStyle::Selected } else { ButtonStyle::Normal })
                    .padding([10, 20, 10, 10])
            })
                .map(Element::<_, _>::from)
                .collect()
        })).id(scrollable::Id::new("scrollable"));

        icolumn![ input, entries ]
            .width(Length::Fill).height(Length::Fill)
            .into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if !self.first_event {
            self.first_event = true;
            log_time("recieved first event");
        }

        // iced::window::fetch_size(f)
        // scrollable::Properties::default().width
        use keyboard::Event::KeyPressed;
        match message {
            Message::Event(event) => match event {
                KeyPressed { key_code: KeyCode::Escape, .. } => return iced::window::close(),
                // TODO: gently scroll window to selected choice
                KeyPressed { key_code: KeyCode::J, modifiers: Modifiers::CTRL } | KeyPressed { key_code: KeyCode::N, modifiers: Modifiers::CTRL } | KeyPressed { key_code: KeyCode::Down, .. } => {
                    self.selected += 1;
                    self.selected = self.selected.min(self.entries.len().saturating_sub(1));
                }
                KeyPressed { key_code: KeyCode::K, modifiers: Modifiers::CTRL } | KeyPressed { key_code: KeyCode::P, modifiers: Modifiers::CTRL }
                | KeyPressed { key_code: KeyCode::Up, .. } => {
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
            Message::FontLoaded(_) => (),
            Message::IconCacheLoaded(icon_cache) => self.icons = icon_cache,
            Message::Entries(entries) => self.entries = entries,
            Message::SenderLoaded(sender) => {
                self.sender = Some(sender);
                self.update_input(self.input.clone(), true); // in case the user typed in before the manager was loaded
            },
            Message::Action(action) => return self.handle_action(action),
        };

        Command::none()
    }
}

impl Keal {
    pub fn update_input(&mut self, input: String, from_user: bool) {
        self.input = input.clone();
        if let Some(sender) = &mut self.sender {
            sender.try_send(async_manager::Event::UpdateInput(input, from_user)).expect("failed to send update input command");
        }
    }

    fn handle_action(&mut self, action: Action) -> Command<Message> {
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
                return iced::window::close();
            }
            Action::PrintAndClose(message) => {
                println!("{message}");
                return iced::window::close();
            }
            Action::Fork => match fork().expect("failed to fork") {
                Fork::Parent(_) => return iced::window::close(),
                Fork::Child => ()
            }
            Action::WaitAndClose => {
                self.manager.with_manager(|m| m.wait());
                return iced::window::close();
            }
        }

        Command::none()
    }
}
