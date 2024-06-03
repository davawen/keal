use std::{os::unix::process::CommandExt, sync::mpsc::{channel, Receiver, Sender, TryRecvError}};

use fork::{fork, Fork};
use raylib::prelude::*;
use nucleo_matcher::Matcher;
use smallvec::SmallVec;

use crate::{icon::{IconCache, Icon}, config::config, plugin::{Action, entry::{Label, OwnedEntry}}, log_time};

pub use styled::Theme;
// use styled::{ButtonStyle, TextStyle};

use self::{match_span::MatchSpan, async_manager::AsyncManager};

mod styled;
mod match_span;
mod async_manager;

type TTFAtlas<'a> = TrueTypeFontAtlas<'a>;

/// Returns a vector of indices (byte offsets) at which the text should wrap, as well as the total height of the text
fn measure_text_wrap(d: &mut DrawHandle, text: &str, max_width: f32, atlas: &mut TTFAtlas, font_size: f32, line_height: f32) -> WrapInfo {
    let max_width = max_width.max(font_size*2.0);

    let mut splits = SmallVec::new();
    let mut height = font_size;

    let mut running_width = 0.0;

    let mut line_start = 0;
    let mut last = 0;
    let mut iter = text.char_indices();
    iter.next();
    for (index, c) in iter {
        let dims = d.measure_text(atlas, &text[last..index], font_size);

        if c == '\n' || running_width + dims.x >= max_width {
            line_start = index;
            running_width = 0.0;

            height += font_size + line_height;
            splits.push(last);
        } 

        running_width += dims.x;
        last = index;
    }

    if line_start < text.len() {
        let dims = d.measure_text(atlas, &text[last..], font_size);
        running_width += dims.x;

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
    wrap_info: Vec<(WrapInfo, Option<WrapInfo>)>,
    total_height: f32
}

impl Entries {
    fn new(list: Vec<OwnedEntry>, rl: &mut Raylib, d: &mut DrawHandle, atlas: &mut TTFAtlas) -> Self {
        let mut this = Self {
            list,
            wrap_info: Vec::new(),
            total_height: 0.0
        };

        this.recalculate(rl, d, atlas);
        this
    }

    /// call this when the screen width changes
    fn recalculate(&mut self, rl: &mut Raylib, d: &mut DrawHandle, font: &mut TTFAtlas) {
        let config = config();

        self.total_height = 0.0;
        self.wrap_info.clear();
        self.wrap_info.extend(self.list.iter().map(|entry| {
            let name = measure_text_wrap(d, &entry.name, rl.get_render_width()/2.0, font, config.font_size, 5.0);
            let mut max_height = name.height;

            let comment_width = rl.get_render_width() - name.width - 10.0 - 20.0 - 10.0; // this removes: name left padding, name-comment inner padding, comment right padding
            let comment = entry.comment.as_ref()
                .map(|comment| measure_text_wrap(d, comment, comment_width, font, config.font_size, 5.0))
                .inspect(|comment| max_height = max_height.max(comment.height));

            self.total_height += max_height + 20.0;

            (name, comment)
        }));
    }
}

pub struct Keal<'a> {
    pub quit: bool,

    // UI state
    input: String,
    /// byte index of the cursor in the text input, None if the input is not selected
    cursor_index: Option<usize>,
    cursor_tick: usize,
    scroll: f32,

    selected: usize,
    hovered_choice: Option<usize>,
    input_hovered: bool,

    old_screen_width: f32,

    // data state
    icons: IconCache,
    atlas: TTFAtlas<'a>,
    atlas_big: TTFAtlas<'a>,

    entries: Entries,
    manager: AsyncManager,

    message_sender: Sender<Message>,
    message_rec: Receiver<Message>
}

#[derive(Debug, Clone)]
pub enum Message {
    // UI events
    Launch(Option<Label>),

    // Worker events
    IconCacheLoaded(IconCache),
    Entries(Vec<OwnedEntry>),
    Action(Action)
}

impl<'a> Keal<'a> {
    pub fn new(rl: &mut Raylib, font: &'a TrueTypeFont) -> Self {
        log_time("initializing app");

        let config = config();

        let (message_sender, message_rec) = channel();

        let atlas = font.atlas(rl, config.font_size);
        let atlas_big = font.atlas(rl, config.font_size * 1.25);
        log_time("finished loading font");

        {
            let message_sender = message_sender.clone();
            std::thread::spawn(move || {
                let icon_cache = IconCache::new(&config.icon_theme);
                let _ = message_sender.send(Message::IconCacheLoaded(icon_cache));
            });
        }

        let manager = AsyncManager::new(Matcher::default(), 50, true, message_sender.clone());

        log_time("finished initializing");

        Keal {
            quit: false,
            input: String::new(),
            cursor_index: Some(0),
            cursor_tick: 0,
            scroll: 0.0,
            selected: 0,
            hovered_choice: None,
            input_hovered: false,
            old_screen_width: 0.0,
            icons: Default::default(),
            atlas,
            atlas_big,
            entries: Default::default(),
            manager,
            message_sender,
            message_rec
        }
    }

    pub fn render(&mut self, rl: &mut Raylib, draw: &mut DrawHandle) {
        let entries = &self.entries;
        let config = config();

        let font = &mut self.atlas;
        let font_size = config.font_size;

        let data = &mut *self.manager.get_data();
        let mut buf = vec![];

        // TODO: scrollbar

        let search_bar_height = (config.font_size*3.25).ceil();
        let mouse = rl.get_mouse_pos();

        self.scroll += rl.get_mouse_wheel_move()*20.0;
        // self.scroll = self.scroll.clamp(rl.get_render_height()-self.entries.total_height - search_bar_height, 0.0);
        self.hovered_choice = None;

        let mut offset_y = search_bar_height + self.scroll;

        for (index, (entry, wrap_info)) in entries.list.iter().zip(entries.wrap_info.iter()).enumerate() {
            let max_height = wrap_info.0.height.max(wrap_info.1.as_ref().map(|x| x.height).unwrap_or(0.0));
            let next_offset_y = offset_y + max_height + 20.0;
            if next_offset_y < 0.0 { 
                offset_y = next_offset_y;
                continue
            }
            if offset_y > rl.get_render_height() { break }

            let selected = self.selected == index;

            if mouse.y >= offset_y && mouse.y < next_offset_y {
                self.hovered_choice = Some(index);
                if !selected {
                    draw.rectangle(0.0, offset_y, rl.get_render_width(), next_offset_y-offset_y, config.theme.hovered_choice_background);
                }
            }
            if selected {
                draw.rectangle(0.0, offset_y, rl.get_render_width(), next_offset_y-offset_y, config.theme.selected_choice_background);
            } 

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
            let mut name_offset_y = offset_y + 10.0;

            for &line_end in &wrap_info.0.splits {
                let text = &entry.name[line_start..line_end];

                let mut offset = 10.0;
                for (span, highlighted) in MatchSpan::new(text, &mut data.matcher, &data.pattern, &mut buf) {
                    let color = match highlighted {
                        false => config.theme.text,
                        true => match selected {
                            false => config.theme.matched_text,
                            true => config.theme.selected_matched_text
                        }
                    };

                    let new_pos = draw.text(font, span, vec2(offset, name_offset_y.ceil()), font_size, color);
                    offset = new_pos.x;
                }

                name_offset_y += config.font_size + 5.0;
                line_start = line_end;
            }


            let mut comment_offset_y = offset_y + 10.0;
            // fill the whole line up
            if let Some(comment) = &entry.comment {
                let wrap_info = wrap_info.1.as_ref().unwrap();

                let mut line_start = 0;
                for &line_end in &wrap_info.splits {
                    let text = &comment[line_start..line_end];

                    draw.text(font, text, vec2(rl.get_render_width() - wrap_info.width - 10.0, comment_offset_y), font_size, config.theme.comment);
                    comment_offset_y += config.font_size + 5.0;
                    line_start = line_end;
                }
            }

            offset_y = next_offset_y;
        }

        // input
        {
            let font = &mut self.atlas_big;
            let text = if self.input.is_empty() && self.cursor_index.is_none() { &config.placeholder_text } else { &self.input };

            let size = config.font_size*1.25;

            let left_padding = config.font_size;
            let baseline = (search_bar_height/2.0 - size/2.0).ceil();

            draw.rectangle(0.0, 0.0, rl.get_render_width(), search_bar_height, config.theme.input_background);
            draw.text(font, &text, vec2(left_padding, baseline), size, config.theme.text);

            if let Some(cursor_index) = self.cursor_index {
                let cursor_position = if self.input.is_empty() {
                    0.0
                } else { draw.measure_text(font, &text[0..cursor_index], size).x };

                if self.cursor_tick % 60 < 30 {
                    draw.rectangle(left_padding + cursor_position - 1.0, baseline, 1.0, size + 5.0, Color::WHITE);
                }
            }

            self.input_hovered = mouse.y >= 0.0 && mouse.y < search_bar_height;
        }
    }

    pub fn update(&mut self, rl: &mut Raylib, draw: &mut DrawHandle) {
        if self.old_screen_width != rl.get_render_width() {
            self.entries.recalculate(rl, draw, &mut self.atlas);
            self.old_screen_width = rl.get_render_width();
        }

        if let Some(hovered_choice) = self.hovered_choice {
            rl.set_mouse_cursor(MouseCursor::PointingHand);

            if rl.is_mouse_button_pressed(MouseButton::Left) {
                self.message_sender.send(Message::Launch(Some(self.entries.list[hovered_choice].label))).expect("message reciever destroyed");
            }
        } else if self.input_hovered {
            rl.set_mouse_cursor(MouseCursor::Ibeam);

            if rl.is_mouse_button_pressed(MouseButton::Left) {
                self.cursor_index = Some(0);
            }
        } else {
            rl.set_mouse_cursor(MouseCursor::Default);
        }

        if rl.is_key_pressed(KeyboardKey::Enter) {
            let _ = self.message_sender.send(Message::Launch(Some(self.entries.list[self.selected].label)));
        }

        if let Some(cursor_index) = &mut self.cursor_index {
            self.cursor_tick += 1;

            let mut modified = false;
            while let Some(ch) = rl.get_char_pressed() {
                self.input.insert(*cursor_index, ch);
                *cursor_index += ch.len_utf8();

                self.cursor_tick = 0;
                modified = true;
            }

            while let Some(key) = rl.get_key_pressed() {
                match key {
                    KeyboardKey::Left if *cursor_index > 0 => {
                        *cursor_index -= 1;
                        while *cursor_index > 0 && !self.input.is_char_boundary(*cursor_index) {
                            *cursor_index -= 1;
                        }
                    }
                    KeyboardKey::Right if *cursor_index < self.input.len() => {
                        *cursor_index += 1;
                        while *cursor_index < self.input.len() && !self.input.is_char_boundary(*cursor_index) {
                            *cursor_index += 1;
                        }
                    }
                    KeyboardKey::Backspace if *cursor_index > 0 => {
                        *cursor_index -= 1;
                        while *cursor_index > 0 && !self.input.is_char_boundary(*cursor_index) {
                            *cursor_index -= 1;
                        }
                        self.input.remove(*cursor_index);
                        modified = true;
                    }
                    _ => ()
                }
                self.cursor_tick = 0;
            }

            if modified {
                self.update_input(true);
            }

        } else {
            self.cursor_tick = 0;
        }

        // KeyPressed { key_code: KeyCode::Escape, .. } => return iced::window::close(),
        let ctrl = rl.is_key_down(KeyboardKey::LeftControl);
        if rl.is_key_pressed(KeyboardKey::Down) || (ctrl && rl.is_key_pressed(KeyboardKey::J)) || (ctrl && rl.is_key_pressed(KeyboardKey::N)) {
            // TODO: gently scroll window to selected choice
            self.selected += 1;
            self.selected = self.selected.min(self.entries.list.len().saturating_sub(1));
        }
        if rl.is_key_pressed(KeyboardKey::Up) || (ctrl && rl.is_key_pressed(KeyboardKey::K)) || (ctrl && rl.is_key_pressed(KeyboardKey::P)) {
            self.selected = self.selected.saturating_sub(1);
        }

        loop {
            let message = match self.message_rec.try_recv() {
                Ok(message) => message,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("manager channel disconnected")
            };

            match message {
                Message::Launch(selected) => {
                    self.manager.send(async_manager::Event::Launch(selected));
                }
                Message::IconCacheLoaded(icon_cache) => self.icons = icon_cache,
                Message::Entries(entries) => self.entries = Entries::new(entries, rl, draw, &mut self.atlas),
                Message::Action(action) => return self.handle_action(action),
            };
        }
    }
}

impl Keal<'_> {
    pub fn update_input(&mut self, from_user: bool) {
        self.manager.send(async_manager::Event::UpdateInput(self.input.clone(), from_user));
    }

    fn handle_action(&mut self, action: Action) /* -> Command<Message> */ {
        match action {
            Action::None => (),
            Action::ChangeInput(new) => {
                self.manager.with_manager(|m| m.kill());
                self.input = new;
                self.update_input(false);
                // return text_input::move_cursor_to_end(text_input::Id::new("query_input"));
            }
            Action::ChangeQuery(new) => {
                let new = self.manager.use_manager(|m| m.current().map(
                    |plugin| format!("{} {}", plugin.prefix, new) 
                )).unwrap_or(new);
                self.input = new;
                self.update_input(false);

                // return text_input::move_cursor_to_end(text_input::Id::new("query_input"));
            }
            Action::Exec(mut command) => {
                let _ = command.0.exec();
                self.quit = true;
            }
            Action::PrintAndClose(message) => {
                println!("{message}");
                self.quit = true;
            }
            Action::Fork => match fork().expect("failed to fork") {
                Fork::Parent(_) => self.quit = true,
                Fork::Child => ()
            }
            Action::WaitAndClose => {
                self.manager.with_manager(|m| m.wait());
                self.quit = true;
            }
        }
    }
}
