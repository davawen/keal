use std::{os::unix::process::CommandExt, cell::RefCell};

use fork::{fork, Fork};
use iced::{Application, executor, Command, widget::{row as irow, text_input, column as icolumn, container, text, Space, scrollable, button, image, svg}, font, Element, Length, subscription, Event, keyboard::{self, KeyCode, Modifiers}};
use nucleo_matcher::{Matcher, pattern::{Pattern, CaseMatching}};

use crate::{icon::{IconCache, Icon}, config::Config, plugin::{PluginManager, Action, entry::Label}};

pub use styled::Theme;
use styled::{ButtonStyle, TextStyle};

use self::{match_span::MatchSpan, cached_manager::CachedManager};

mod styled;
mod match_span;
mod cached_manager;

pub struct Keal<'a> {
    // UI state
    input: String,
    selected: usize,

    // data state
    query: String,
    pattern: Pattern,
    matcher: RefCell<Matcher>,
    icons: IconCache,
    config: Config,
    manager: CachedManager<'a>
}

#[derive(Debug, Clone)]
pub enum Message {
    FontLoaded(Result<(), font::Error>),
    TextInput(String),
    Launch(Option<Label>),
    Event(keyboard::Event),
    IconCacheLoaded(IconCache)
}

// TODO: fuzzel-like often launched applications

#[derive(Default)]
pub struct Flags(pub Config, pub PluginManager);

/// unfortunately cannot be implemented as a function, since it's not possible (yet) to partially borrow struct fields
macro_rules! get_entries {
    ($self:expr, $manager:expr) => {
        $manager.get_entries(&($self).config, &mut ($self).matcher.borrow_mut(), &($self).pattern, 50, ($self).config.usage_frequency)
    };
}

impl Application for Keal<'_> {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = Flags;

    fn new(Flags(config, manager): Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let focus = text_input::focus(text_input::Id::new("query_input")); // focus input on start up

        let iosevka = include_bytes!("../../public/iosevka-regular.ttf");
        let iosevka = font::load(iosevka.as_slice()).map(Message::FontLoaded);

        let icon_theme = config.icon_theme.clone();
        let load_icons = Command::perform(async move {
            IconCache::new(&icon_theme)
        }, Message::IconCacheLoaded);

        let command = Command::batch(vec![iosevka, focus, load_icons]);

        let mut matcher = Matcher::default();
        let manager = CachedManager::new(manager, |m| m.get_entries(&config, &mut matcher, &Pattern::default(), 50, true));

        (Keal {
            input: String::new(),
            selected: 0,
            query: String::new(),
            pattern: Pattern::default(),
            matcher: matcher.into(),
            icons: IconCache::default(),
            config,
            manager
        }, command)
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
        let entries = self.manager.entries();

        let input = text_input(&self.config.placeholder_text, &self.input)
            .on_input(Message::TextInput)
            .on_submit(Message::Launch(entries.get(self.selected).map(|e| e.label)))
            .size(self.config.font_size * 1.25).padding(self.config.font_size)
            .id(text_input::Id::new("query_input"));

        let input = container(input)
            .width(Length::Fill);

        let mut matcher = self.matcher.borrow_mut();
        let mut buf = vec![];

        let entries = scrollable(icolumn({
            entries.iter().enumerate().map(|(index, entry)| {
                let selected = self.selected == index;

                let mut item = irow(vec![]);

                if let Some(icon) = entry.icon {
                    if let Some(icon) = self.icons.get(icon) {
                        let element: Element<_, _> = match icon {
                            Icon::Svg(path) => svg(svg::Handle::from_path(path)).width(self.config.font_size).height(self.config.font_size).into(),
                            Icon::Other(path) => image(path).width(self.config.font_size).height(self.config.font_size).into()
                        };
                        item = item.push(container(element).padding(4));
                    }
                }

                for (span, highlighted) in MatchSpan::new(entry.name, &mut matcher, &self.pattern, &mut buf) {
                    item = item.push(text(span).size(self.config.font_size).style(
                        match highlighted {
                            false => TextStyle::Normal,
                            true => TextStyle::Matched { selected },
                        }
                    ));
                }

                item = item.push(Space::with_width(Length::Fill)); // fill the whole line up
                if let Some(comment) = entry.comment {
                    item = item.push(Space::with_width(5.0)); // minimum amount of space between name and comment
                    item = item.push(
                        text(comment)
                            .size(self.config.font_size)
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
        // iced::window::fetch_size(f)
        // scrollable::Properties::default().width
        use keyboard::Event::KeyPressed;
        match message {
            Message::FontLoaded(_) => (),
            Message::TextInput(input) => return self.update_input(input, true),
            Message::Event(event) => match event {
                KeyPressed { key_code: KeyCode::Escape, .. } => return iced::window::close(),
                // TODO: gently scroll window to selected choice
                KeyPressed { key_code: KeyCode::J, modifiers: Modifiers::CTRL } | KeyPressed { key_code: KeyCode::N, modifiers: Modifiers::CTRL } | KeyPressed { key_code: KeyCode::Down, .. } => {
                    self.selected += 1;
                    self.selected = self.selected.min(self.manager.entries().len().saturating_sub(1));
                }
                KeyPressed { key_code: KeyCode::K, modifiers: Modifiers::CTRL } | KeyPressed { key_code: KeyCode::P, modifiers: Modifiers::CTRL }
                | KeyPressed { key_code: KeyCode::Up, .. } => {
                    self.selected = self.selected.saturating_sub(1);
                }
                _ => ()
            }
            Message::Launch(selected) => {
                let action = self.manager.use_manager(
                    |manager| manager.launch(&self.config, &self.query, selected),
                    |manager| get_entries!(self, manager)
                );
                return self.handle_action(action);
            }
            Message::IconCacheLoaded(icon_cache) => self.icons = icon_cache
        };

        Command::none()
    }
}

impl Keal<'_> {
    pub fn update_input(&mut self, input: String, from_user: bool) -> Command<Message> {
        self.input = input;

        let mut action = Action::None;
        self.manager.modify(|manager| {
            let (query, act) = manager.update_input(&self.config, &self.input, from_user);
            action = act;
            self.pattern.reparse(&query, CaseMatching::Ignore);
            self.query = query;
            get_entries!(self, manager)
        });

        self.handle_action(action)
    }

    fn handle_action(&mut self, action: Action) -> Command<Message> {
        match action {
            Action::None => (),
            Action::ChangeInput(new) => {
                self.manager.use_manager(
                    |manager| manager.kill(),
                    |manager| get_entries!(self, manager)
                );
                let c = self.update_input(new, false);
                return Command::batch([c, text_input::move_cursor_to_end(text_input::Id::new("query_input"))]);
            }
            Action::ChangeQuery(new) => {
                let new = self.manager.manager().current()
                    .map(|plugin| format!("{} {}", plugin.prefix, new))
                    .unwrap_or(new);

                let c = self.update_input(new, false);
                return Command::batch([c, text_input::move_cursor_to_end(text_input::Id::new("query_input"))]);
            }
            Action::Exec(mut command) => {
                let _ = command.exec();
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
                self.manager.use_manager(
                    |manager| manager.wait(),
                    |manager| get_entries!(self, manager)
                );
                return iced::window::close();
            }
        }

        Command::none()
    }
}
