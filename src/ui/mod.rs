use std::io::Write;

use fuzzy_matcher::skim::SkimMatcherV2;
use iced::{Application, Theme, executor, Command, widget::{row as irow, text_input, column as icolumn, container, text, Space, scrollable, button}, theme, font, Element, Length, color, subscription, Event, keyboard::{self, KeyCode, Modifiers}};

use crate::search::{self, plugin::{get_plugins, PluginExecution}, create_entries, EntryTrait, Entry};

mod styled;

type Matcher = SkimMatcherV2;

pub struct Keal {
    input: String,
    filter: String,
    matcher: Matcher,
    plugins: search::plugin::Plugins,
    entries: Vec<search::Entry>,
    shown: Shown,
    selected: usize
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
    type Flags = ();

    fn new(_: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let plugins = get_plugins();
        let entries = create_entries(&plugins);
        let filtered = entries.iter().take(50).enumerate().map(|(i, _)| (i, 0)).collect();
        let this = Keal {
            input: String::new(), filter: String::new(),
            matcher: Matcher::default(),
            plugins, entries,
            shown: Shown::Entries(filtered),
            selected: 0
        };

        let focus = text_input::focus(text_input::Id::new("filter_input")); // focus input on start up

        let iosevka = include_bytes!("../../public/iosevka-regular.ttf");
        let iosevka = font::load(iosevka.as_slice()).map(Message::FontLoaded);

        (this, Command::batch(vec![iosevka, focus]))
    }

    fn title(&self) -> String {
        "Keal".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        use keyboard::Event::KeyPressed;
        match message {
            Message::FontLoaded(_) => (),
            Message::TextInput(input) => self.update_input(input),
            Message::Event(event) => match event {
                KeyPressed { key_code: KeyCode::Escape, .. } => return iced::window::close(),
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
                Shown::Plugin { execution, filtered } => match &execution.entries[filtered[selected].0] {
                    Entry::FieldEntry(field) => {
                        let _ = writeln!(execution.stdin, "{}", field.name());
                        let _ = execution.child.wait();
                        return iced::window::close();
                    }
                    _ => unreachable!("something went terribly wrong")
                }
                Shown::Entries(filtered) => match &self.entries[filtered[selected].0] {
                    Entry::PluginEntry(plugin) => {
                        let input = format!("{} ", plugin.name());
                        self.update_input(input);
                        return text_input::move_cursor_to_end(text_input::Id::new("filter_input"));
                    }
                    Entry::DesktopEntry(_app) => {
                        todo!("launch application")
                    }
                    _ => unreachable!("something went terribly wrong")
                }
            }
        };

        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        subscription::events_with(|event, _status| match event {
            Event::Keyboard(k) => Some(Message::Event(k)),
            _ => None
        })
    }

    fn theme(&self) -> Self::Theme {
        Theme::Custom(Box::new(theme::Custom::new(theme::Palette {
            text: color!(0xcad3f5),
            background: color!(0x24273a),
            danger: color!(0xed8796),
            primary: color!(0xf5a97f),
            success: color!(0xa6da95)
        })))
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        let input = text_input("search your dreams!", &self.input)
            .on_input(Message::TextInput)
            .on_submit(Message::Launch(self.selected))
            .size(20).padding(16)
            .style(theme::TextInput::Custom(Box::new(styled::Input)))
            .id(text_input::Id::new("filter_input"));

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

                let mut item = vec![];
                for (span, highlighted) in entry.fuzzy_match_span(&self.matcher, &self.filter) {
                    let style = match highlighted {
                        false => theme::Text::Default,
                        true => theme::Text::Color(
                            if selected { *styled::SELECTED_MATCHED_TEXT_COLOR } else { *styled::MATCHED_TEXT_COLOR }
                        )
                    };

                    item.push(text(span).style(style).into());
                }
                item.push(Space::with_width(Length::Fill).into());
                item.push(text(entry.comment().unwrap_or("")).style(theme::Text::Color(*styled::COMMENT_COLOR)).into());

                button(irow(item))
                    .on_press(Message::Launch(index))
                    .style(
                        if selected { theme::Button::custom(styled::SelectedItem) }
                        else { theme::Button::custom(styled::Item) })
                    .padding([10, 20, 10, 10])
            })
                .map(Element::from)
                .collect()
        }));

        icolumn![ input, entries ]
            .width(Length::Fill).height(Length::Fill)
            .into()
    }
}

impl Keal {
    fn update_input(&mut self, input: String) {
        self.input = input;

        // launch or stop plugin execution depending on new state of filter
        // if in plugin mode, remove plugin prefix from filter
        self.filter = match (self.plugins.filter_starts_with_plugin(&self.input), &self.shown) {
            (Some((plugin, remainder)), Shown::Entries(_)) => { // launch plugin
                self.shown = Shown::Plugin { 
                    execution: plugin.generate(),
                    filtered: Vec::new() // filter happens right after
                };
                remainder.to_owned()
            }
            (None, Shown::Plugin { .. }) => { // stop plugin
                self.shown = Shown::Entries(Vec::new());
                self.input.clone()
            }
            (Some((_, remainer)), Shown::Plugin { .. }) => remainer.to_owned(),
            (None, Shown::Entries(_)) => self.input.clone()
        };

        match &mut self.shown {
            Shown::Entries(filtered) => *filtered = search::filter_entries(&self.matcher, &self.entries, &self.filter, 50),
            Shown::Plugin { execution, filtered } =>
                *filtered = search::filter_entries(&self.matcher, &execution.entries, &self.filter, 50)
        }
    }
}
