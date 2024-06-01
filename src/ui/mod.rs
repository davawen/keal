use std::os::unix::process::CommandExt;

use fork::{fork, Fork};
// use iced::{Application, executor, Command, widget::{row as irow, text_input, column as icolumn, container, text, Space, scrollable, button, image, svg}, font, Element, Length, subscription, Event, keyboard::{self, KeyCode, Modifiers}, futures::channel::mpsc};
use macroquad::prelude::*;
use nucleo_matcher::Matcher;
use smallvec::SmallVec;

use crate::{icon::{IconCache, Icon}, config::config, plugin::{Action, entry::{Label, OwnedEntry}}, log_time};

pub use styled::Theme;
// use styled::{ButtonStyle, TextStyle};

use self::{match_span::MatchSpan, async_manager::AsyncManager};

mod styled;
mod match_span;
mod async_manager;

/// Returns a vector of indices (byte offsets) at which the text should wrap, as well as the total height of the text
fn measure_text_wrap(text: &str, max_width: f32, font: Option<&Font>, font_size: f32, line_height: f32) -> WrapInfo {
    let max_width = max_width.max(font_size*2.0);

    let font_size_ratio = font_size / ((font_size as u16) as f32);

    let mut splits = SmallVec::new();
    let mut height = font_size;

    let mut running_width = 0.0;

    let mut line_start = 0;
    let mut last = 0;
    let mut iter = text.char_indices();
    iter.next();
    for (index, c) in iter {
        let dims = measure_text(&text[last..index], font, font_size as u16, font_size_ratio);

        if c == '\n' || running_width + dims.width >= max_width {
            line_start = index;
            running_width = 0.0;

            height += font_size + line_height;
            splits.push(last);
        } 

        running_width += dims.width;
        last = index;
    }

    if line_start < text.len() {
        let dims = measure_text(&text[last..], font, font_size as u16, font_size_ratio);
        running_width += dims.width;

        splits.push(text.len());
    }

    let width = if line_start == 0 { running_width } else { max_width };

    WrapInfo { splits, width, height }
}

struct WrapInfo {
    splits: SmallVec<[usize; 8]>,
    width: f32,
    height: f32
}

#[derive(Default)]
struct Entries {
    list: Vec<OwnedEntry>,
    /// info for entry.name and entry.comment (optional)
    wrap_info: Vec<(WrapInfo, Option<WrapInfo>)>
}

impl Entries {
    fn new(list: Vec<OwnedEntry>) -> Self {
        let mut this = Self {
            list,
            wrap_info: Vec::new()
        };

        this.recalculate();
        this
    }

    /// call this when the screen width changes
    fn recalculate(&mut self) {
        let config = config();

        self.wrap_info.clear();
        self.wrap_info.extend(self.list.iter().map(|entry| {
            let name = measure_text_wrap(&entry.name, screen_width()/2.0, None, config.font_size, 5.0);

            let comment_width = screen_width() - name.width - 10.0 - 20.0 - 10.0; // this removes: name left padding, name-comment inner padding, comment right padding
            let comment = entry.comment.as_ref()
                .map(|comment| measure_text_wrap(comment, comment_width, None, config.font_size, 5.0));

            (name, comment)
        }));
    }
}

pub struct Keal {
    // UI state
    input: String,
    selected: usize,
    scroll: f32,

    old_screen_width: f32,

    // data state
    icons: IconCache,

    entries: Entries,
    manager: AsyncManager,

    first_event: bool
}

#[derive(Debug, Clone)]
pub enum Message {
    // UI events
    TextInput(String),
    Launch(Option<Label>),
    Event(KeyCode),

    // Worker events
    IconCacheLoaded(IconCache),
    Entries(Vec<OwnedEntry>),
    Action(Action)
}

impl Keal {
    pub fn new() -> Self {
        log_time("initializing app");

        let config = config();

        let iosevka = include_bytes!("../../public/iosevka-regular.ttf");
        let _iosevka = load_ttf_font_from_bytes(iosevka).expect("failed to load font");

        let manager = AsyncManager::new(Matcher::default(), 50, true);

        log_time("finished initializing");

        Keal {
            input: String::new(),
            selected: 0,
            scroll: 0.0,
            old_screen_width: 0.0,
            icons: IconCache::new(&config.icon_theme),
            entries: Default::default(),
            manager,
            first_event: false
        }
    }

    pub fn render(&mut self) {
        let entries = &self.entries;
        let config = config();

        let data = &mut *self.manager.get_data();
        let mut buf = vec![];

        // TODO: scrollbar

        self.scroll += mouse_wheel().1;

        let mut offset_y = (config.font_size*3.25).ceil() + 10.0 + self.scroll*20.0;

        let font_size_ratio = config.font_size / config.font_size.floor();

        for (index, (entry, wrap_info)) in entries.list.iter().zip(entries.wrap_info.iter()).enumerate() {
            let max_height = wrap_info.0.height.max(wrap_info.1.as_ref().map(|x| x.height).unwrap_or(0.0));
            if offset_y + max_height < 0.0 { 
                offset_y += max_height + config.font_size + 10.0;
                continue
            }
            if offset_y > screen_height() { break }

            let selected = self.selected == index;

            // if let Some(icon) = &entry.icon {
            //     if let Some(icon) = self.icons.get(icon) {
            //         let element: Element<_, _> = match icon {
            //             Icon::Svg(path) => svg(svg::Handle::from_path(path)).width(config.font_size).height(config.font_size).into(),
            //             Icon::Other(path) => image(path).width(config.font_size).height(config.font_size).into()
            //         };
            //         item = item.push(container(element).padding(4));
            //     }
            // }

            let mut line_start = 0;
            let mut name_offset_y = offset_y;
            for &line_end in &wrap_info.0.splits {
                let text = &entry.name[line_start..line_end];
                let mut offset = 0.0;
                for (span, highlighted) in MatchSpan::new(text, &mut data.matcher, &data.pattern, &mut buf) {
                    let dims = measure_text(span, None, config.font_size as u16, font_size_ratio);

                    let color = match highlighted {
                        false => config.theme.text,
                        true => match selected {
                            false => config.theme.matched_text,
                            true => config.theme.selected_matched_text
                        }
                    };

                    draw_text(span, offset, offset_y + config.font_size, config.font_size, color);
                    offset += dims.width;
                }

                name_offset_y += config.font_size + 5.0;
                line_start = line_end;
            }


            let mut comment_offset_y = offset_y;
            // fill the whole line up
            if let Some(comment) = &entry.comment {
                let wrap_info = wrap_info.1.as_ref().unwrap();

                let mut line_start = 0;
                for &line_end in &wrap_info.splits {
                    let text = &comment[line_start..line_end];

                    draw_text(text, screen_width() - wrap_info.width - 10.0, comment_offset_y + config.font_size, config.font_size, config.theme.comment);
                    comment_offset_y += config.font_size + 5.0;
                    line_start = line_end;
                }
            }

            offset_y += max_height + config.font_size + 20.0;

            // .on_press(Message::Launch(Some(entry.label)))
        }

        let height = (config.font_size * 3.25).ceil();
        let text = if self.input.is_empty() { &config.placeholder_text } else { &self.input };

        let size = config.font_size*1.25;
        let dims = measure_text(text, None, size as u16, size/size.floor());

        draw_rectangle(0.0, 0.0, screen_width(), height, config.theme.input_background);
        draw_text(&text, config.font_size, height/2.0 - dims.offset_y + dims.height, size, config.theme.text);
    }

    pub fn update(&mut self) {
        if self.old_screen_width != screen_width() {
            self.entries.recalculate();
            self.old_screen_width = screen_width();
        }

        let Some(message) = self.manager.poll() else { return };

        match message {
            // Message::Event(event) => match event {
            //     KeyPressed { key_code: KeyCode::Escape, .. } => return iced::window::close(),
            //     // TODO: gently scroll window to selected choice
            //     KeyPressed { key_code: KeyCode::J, modifiers: Modifiers::CTRL } | KeyPressed { key_code: KeyCode::N, modifiers: Modifiers::CTRL } | KeyPressed { key_code: KeyCode::Down, .. } => {
            //         self.selected += 1;
            //         self.selected = self.selected.min(self.entries.len().saturating_sub(1));
            //     }
            //     KeyPressed { key_code: KeyCode::K, modifiers: Modifiers::CTRL } | KeyPressed { key_code: KeyCode::P, modifiers: Modifiers::CTRL }
            //     | KeyPressed { key_code: KeyCode::Up, .. } => {
            //         self.selected = self.selected.saturating_sub(1);
            //     }
            //     _ => ()
            // }
            Message::Event(_) => (),
            Message::TextInput(input) => self.update_input(input, true),
            Message::Launch(selected) => {
                self.manager.send(async_manager::Event::Launch(selected));
            }
            Message::IconCacheLoaded(icon_cache) => self.icons = icon_cache,
            Message::Entries(entries) => self.entries = Entries::new(entries),
            Message::Action(action) => return self.handle_action(action),
        };
    }
}

impl Keal {
    pub fn update_input(&mut self, input: String, from_user: bool) {
        self.input = input.clone();
        self.manager.send(async_manager::Event::UpdateInput(input, from_user));
    }

    fn handle_action(&mut self, action: Action) /* -> Command<Message> */ {
        match action {
            Action::None => (),
            Action::ChangeInput(new) => {
                self.manager.with_manager(|m| m.kill());
                self.update_input(new, false);
                // return text_input::move_cursor_to_end(text_input::Id::new("query_input"));
            }
            Action::ChangeQuery(new) => {
                let new = self.manager.use_manager(|m| m.current().map(
                    |plugin| format!("{} {}", plugin.prefix, new) 
                )).unwrap_or(new);
                self.update_input(new, false);

                // return text_input::move_cursor_to_end(text_input::Id::new("query_input"));
            }
            Action::Exec(mut command) => {
                let _ = command.0.exec();
                // return iced::window::close();
            }
            Action::PrintAndClose(message) => {
                println!("{message}");
                // return iced::window::close();
            }
            Action::Fork => match fork().expect("failed to fork") {
                Fork::Parent(_) => (),//return iced::window::close(),
                Fork::Child => ()
            }
            Action::WaitAndClose => {
                self.manager.with_manager(|m| m.wait());
                // return iced::window::close();
            }
        }
    }
}
