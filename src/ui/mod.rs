use std::{ffi::{CStr, CString}, os::unix::process::CommandExt, sync::mpsc::{channel, Receiver, Sender, TryRecvError}};

use fork::{fork, Fork};
use raylib::prelude::*;
use nucleo_matcher::Matcher;
use smallvec::SmallVec;

use crate::{config::config, icon::{Icon, IconCache, IconPath}, log_time, plugin::{entry::{Label, OwnedEntry}, Action}};

pub use styled::Theme;
// use styled::{ButtonStyle, TextStyle};

use self::{match_span::MatchSpan, async_manager::AsyncManager};

mod styled;
mod match_span;
mod async_manager;

pub type TTFCache = TrueTypeFontCache;

fn is_key_pressed_repeated(rl: &mut Raylib, key: Key) -> bool {
    is_key_pressed(rl, key) || is_key_pressed_again(rl, key)
}

/// Returns the index of the unicode character to the left of the given index
/// Saturates at the left edge of the string
fn floor_char_boundary(s: &str, mut index: usize) -> usize {
    if index == 0 { return 0 }

    index -= 1;
    while index > 0 && !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// Returns the index of the unicode character to the right of the given index
/// Saturates at the string's length
/// Caution: this means the returned index can be out of bounds
fn ceil_char_boundary(s: &str, mut index: usize) -> usize {
    if index >= s.len() { return s.len() }

    index += 1;
    while index < s.len() && !s.is_char_boundary(index) {
        index += 1;
    }
    index
}

/// Returns the index of the first character left of the given index
/// before a character that isn't an alphanumeric,
/// skipping any non-alphanumeric characters at the start.
fn floor_word_boundary(s: &str, mut index: usize) -> usize {
    let is_alphanum = |idx| s[idx..].chars().next().unwrap().is_alphanumeric();

    // skip non-alphanumeric characters at the start
    loop {
        index = floor_char_boundary(s, index);
        if index == 0 { return index };

        if is_alphanum(index) { break; }
    }

    loop {
        let next = floor_char_boundary(s, index);
        if next == 0 { return next }

        if !is_alphanum(next) { break index }

        index = next;
    }
}

/// Returns the index of the first character right of the given index
/// before a character that isn't an alphanumeric
/// skipping any non-alphanumeric characters at the start.
fn ceil_word_boundary(s: &str, mut index: usize) -> usize {
    let is_alphanum = |idx| s[idx..].chars().next().unwrap().is_alphanumeric();

    // skip non-alphanumeric characters at the start
    loop {
        index = ceil_char_boundary(s, index);
        if index == s.len() { return index };

        if is_alphanum(index) { break; }
    }

    loop {
        index = ceil_char_boundary(s, index);
        if index == s.len() { return index }

        if !is_alphanum(index) { break index }
    }
}


/// order of border radius is: `[top-left, top-right, bot-left, bot-right]`
fn draw_rectangle_rounded(rl: &mut DrawHandle, x: f32, y: f32, w: f32, h: f32, mut borders: [f32; 4], color: Color) {
    for radius in &mut borders {
        *radius = radius.min(w).min(h)
    }

    let top_width = w - borders[0] - borders[1];
    let bot_width = w - borders[2] - borders[3];

    let left_height = h - borders[0] - borders[2];
    let right_height = h - borders[1] - borders[3];

    let pad_top = borders[0].max(borders[1]);
    let pad_bot = borders[2].max(borders[3]);
    let pad_left = borders[0].max(borders[2]);
    let pad_right = borders[1].max(borders[3]);

    draw_rectangle(rl, x + pad_left, y + pad_top, w - pad_left - pad_right, h - pad_top - pad_bot, color);

    draw_rectangle(rl, x + borders[0], y, top_width, pad_top, color);
    draw_rectangle(rl, x + borders[2], y + h - pad_bot, bot_width, pad_bot, color);

    draw_rectangle(rl, x, y + borders[0], pad_left, left_height, color);
    draw_rectangle(rl, x + w - pad_right, y + borders[1], pad_right, right_height, color);

    draw_circle(rl, x + borders[0], y + borders[0], borders[0], color);
    draw_circle(rl, x + w - borders[1], y + borders[1], borders[1], color);
    draw_circle(rl, x + borders[2], y + h - borders[2], borders[2], color);
    draw_circle(rl, x + w - borders[3], y + h - borders[3], borders[3], color);
}


/// Returns a vector of indices (byte offsets) at which the text should wrap, as well as the total height of the text
fn measure_text_wrap(text: &str, max_width: f32, atlas: &TTFCache, font_size: f32, line_height: f32) -> WrapInfo {
    let max_width = max_width.max(font_size*2.0);

    let mut splits = SmallVec::new();
    let mut height = font_size;

    let mut running_width = 0.0;

    let mut line_start = 0;
    let mut last = 0;
    let mut iter = text.char_indices();
    iter.next();
    for (index, c) in iter {
        let dims = measure_text(atlas, &text[last..index], font_size);

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
        let dims = measure_text(atlas, &text[last..], font_size);
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
    fn new(list: Vec<OwnedEntry>, rl: &mut Raylib, atlas: &TTFCache) -> Self {
        let mut this = Self {
            list,
            wrap_info: Vec::new(),
            total_height: 0.0
        };

        this.recalculate(rl, atlas);
        this
    }

    /// call this when the screen width changes
    fn recalculate(&mut self, rl: &mut Raylib, font: &TTFCache) {
        let config = config();

        self.total_height = 0.0;
        self.wrap_info.clear();
        self.wrap_info.extend(self.list.iter().map(|entry| {
            let icon_width = entry.icon.as_ref().map(|_| config.font_size + 4.0).unwrap_or_default();

            let name = measure_text_wrap(&entry.name, get_screen_width(rl)/2.0 - icon_width, font, config.font_size, 5.0);
            let mut max_height = name.height;

            let comment_width = get_screen_width(rl) - name.width - icon_width - 10.0 - 20.0 - 10.0; // this removes: name left padding, name-comment inner padding, comment right padding
            let comment = entry.comment.as_ref()
                .map(|comment| measure_text_wrap(comment, comment_width, font, config.font_size, 5.0))
                .inspect(|comment| max_height = max_height.max(comment.height));

            self.total_height += max_height + 20.0;

            (name, comment)
        }));
    }
}

pub struct Keal {
    // UI state
    input: String,
    /// byte index of the cursor in the text input, None if the input is not selected
    cursor_index: Option<usize>,
    cursor_tick: usize,
    /// byte indices of the start and end ranges of the selection
    select_range: Option<(usize, usize)>,
    scroll: f32,

    selected: usize,
    hovered_choice: Option<usize>,
    input_hovered: bool,

    old_screen_width: f32,

    rendered_icons: std::collections::HashMap<IconPath, Option<Texture>>,

    // data state
    icons: IconCache,
    font: TrueTypeFontCache,

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

impl Keal {
    pub fn new(font: TrueTypeFontCache) -> Self {
        log_time("initializing app");

        let config = config();

        let (message_sender, message_rec) = channel();

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
            input: String::new(),
            cursor_index: Some(0),
            cursor_tick: 0,
            select_range: None,
            scroll: 0.0,
            selected: 0,
            hovered_choice: None,
            input_hovered: false,
            old_screen_width: 0.0,
            rendered_icons: Default::default(),
            icons: Default::default(),
            font,
            entries: Default::default(),
            manager,
            message_sender,
            message_rec
        }
    }

    pub fn render(&mut self, rl: &mut DrawHandle) {
        let entries = &self.entries;
        let config = config();

        let font = &self.font;
        let font_size = config.font_size;

        let data = &mut *self.manager.get_data();
        let mut buf = vec![];

        // TODO: scrollbar

        let search_bar_height = (config.font_size*3.25).ceil();
        let mouse = get_mouse_pos(rl);

        self.scroll -= get_mouse_wheel_move(rl)*20.0;
        self.scroll = self.scroll.clamp(0.0, (self.entries.total_height - get_screen_height(rl) + search_bar_height).max(0.0));
        self.hovered_choice = None;

        let mut offset_y = search_bar_height - self.scroll;

        for (index, (entry, wrap_info)) in entries.list.iter().zip(entries.wrap_info.iter()).enumerate() {
            let max_height = wrap_info.0.height.max(wrap_info.1.as_ref().map(|x| x.height).unwrap_or(0.0));
            let next_offset_y = offset_y + max_height + 20.0;
            if next_offset_y < search_bar_height { 
                offset_y = next_offset_y;
                continue
            }
            if offset_y > get_screen_height(rl) { break }

            let selected = self.selected == index;

            let mut rectangle_color = config.theme.choice_background;
            if mouse.y >= offset_y && mouse.y < next_offset_y {
                self.hovered_choice = Some(index);
                rectangle_color = config.theme.hovered_choice_background;
            }
            if selected { rectangle_color = config.theme.selected_choice_background; } 

            draw_rectangle(rl, 0.0, offset_y, get_screen_width(rl), next_offset_y-offset_y, rectangle_color);

            let mut icon_offset = 10.0;

            if let Some(icon_path) = &entry.icon {
                if let Some(rendered) = self.rendered_icons.get(icon_path) {
                    if let Some(rendered) = rendered {
                        draw_texture_ex(rl, rendered, vec2(icon_offset, offset_y + 10.0), 0.0, config.font_size / rendered.width() as f32, Color::WHITE);
                        icon_offset += config.font_size + 4.0;
                    }
                } else if let Some(icon) = self.icons.get(icon_path) {
                    match icon {
                        Icon::Svg(path) | Icon::Other(path) => {
                            let img = Texture::load(rl, path).unwrap_or_else(|e| {
                                eprintln!("failed to open icon: {e}");
                                None
                            });
                            let img = img.map(|mut i| { i.set_texture_filter(TextureFilter::Bilinear); i });
                            self.rendered_icons.insert(icon_path.clone(), img);
                        }
                    };
                }
            }

            let mut line_start = 0;
            let mut name_offset_y = offset_y + 10.0;

            for &line_end in &wrap_info.0.splits {
                let text = &entry.name[line_start..line_end];

                let mut offset = icon_offset;
                for (span, highlighted) in MatchSpan::new(text, &mut data.matcher, &data.pattern, &mut buf) {
                    let color = match highlighted {
                        false => config.theme.text,
                        true => match selected {
                            false => config.theme.matched_text,
                            true => config.theme.selected_matched_text
                        }
                    };

                    let new_pos = draw_text(rl, font, span, vec2(offset, name_offset_y.ceil()), font_size, color);
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

                    draw_text(rl, font, text, vec2(get_screen_width(rl) - wrap_info.width - 10.0, comment_offset_y), font_size, config.theme.comment);
                    comment_offset_y += config.font_size + 5.0;
                    line_start = line_end;
                }
            }

            offset_y = next_offset_y;
        }

        // input
        {
            let text = if self.input.is_empty() && self.cursor_index.is_none() { &config.placeholder_text } else { &self.input };

            let size = config.font_size*1.25;

            let left_padding = config.font_size;
            let baseline = (search_bar_height/2.0 - size/2.0).ceil();

            draw_rectangle_rounded(rl, 0.0, 0.0, get_screen_width(rl), search_bar_height, [5.0, 5.0, 0.0, 0.0], config.theme.input_background);
            draw_text(rl, font, &text, vec2(left_padding, baseline), size, config.theme.text);

            if let Some((start, end)) = self.select_range {
                let start_pos = if self.input.is_empty() { 0.0 } else { measure_text(font, &text[0..start], size).x };
                let end_pos = if self.input.is_empty() { 0.0 } else { measure_text(font, &text[0..end], size).x };
                draw_rectangle(rl, left_padding + start_pos - 1.0, baseline, end_pos - start_pos + 2.0, size + 5.0, config.theme.input_selection);
            } else if let Some(cursor_index) = self.cursor_index {
                let cursor_position = if self.input.is_empty() { 0.0 } else { measure_text(font, &text[0..cursor_index], size).x };

                if self.cursor_tick % 60 < 30 {
                    draw_rectangle(rl, left_padding + cursor_position - 1.0, baseline, 1.0, size + 5.0, Color::WHITE);
                }
            }

            self.input_hovered = mouse.y >= 0.0 && mouse.y < search_bar_height;
        }
    }

    pub fn update(&mut self, rl: &mut Raylib) {
        if self.old_screen_width != get_screen_width(rl) {
            self.entries.recalculate(rl, &self.font);
            self.old_screen_width = get_screen_width(rl);
        }

        if let Some(hovered_choice) = self.hovered_choice {
            set_mouse_cursor(rl, MouseCursor::PointingHand);

            if is_mouse_button_pressed(rl, MouseButton::Left) {
                self.message_sender.send(Message::Launch(Some(self.entries.list[hovered_choice].label))).expect("message reciever destroyed");
            }
        } else if self.input_hovered {
            set_mouse_cursor(rl, MouseCursor::Ibeam);

            if is_mouse_button_pressed(rl, MouseButton::Left) {
                self.cursor_index = Some(0);
            }
        } else {
            set_mouse_cursor(rl, MouseCursor::Default);
        }

        if is_key_pressed(rl, Key::Enter) {
            let _ = self.message_sender.send(Message::Launch(Some(self.entries.list[self.selected].label)));
        }

        let ctrl = is_key_down(rl, Key::LeftControl) || is_key_down(rl, Key::RightControl);
        let shift = is_key_down(rl, Key::LeftShift) || is_key_down(rl, Key::RightShift);

        if let Some(cursor_index) = &mut self.cursor_index {
            self.cursor_tick += 1;

            let mut modified = false;
            while let Some(ch) = get_char_pressed(rl) {
                if let Some((start, end)) = self.select_range { // remove selected text
                    *cursor_index = start;
                    self.input.drain(start..end);
                    self.select_range = None;
                }

                self.input.insert(*cursor_index, ch);
                *cursor_index += ch.len_utf8();

                self.cursor_tick = 0;
                modified = true;
            }

            if ctrl {
                if is_key_pressed(rl, Key::A) {
                    self.select_range = Some((0, self.input.len()));
                }
                if is_key_pressed(rl, Key::C) {
                    if let Some((start, end)) = self.select_range {
                        let text = &self.input[start..end];
                        set_clipboard_text(rl, &CString::new(text).unwrap());
                    }
                }
                if is_key_pressed(rl, Key::X) {
                    if let Some((start, end)) = self.select_range {
                        *cursor_index = start; // in case we expanded the selection to the right
                        self.select_range = None;

                        let mut text = self.input.drain(start..end).collect::<String>().into_bytes();
                        text.push(0);
                        set_clipboard_text(rl, CStr::from_bytes_until_nul(&text).unwrap());
                        modified = true;
                    }
                }
                if is_key_pressed(rl, Key::V) {
                    if let Some((start, end)) = self.select_range {
                        *cursor_index = start; // in case we expanded the selection to the right
                        self.input.drain(start..end);
                        self.select_range = None;
                        modified = true;
                    }

                    match get_clipboard_text(rl).to_str() {
                        Ok(text) if !text.is_empty() => {
                            self.input.insert_str(*cursor_index, text);
                            *cursor_index += text.len();
                            modified = true;
                        }
                        _ => (),
                    }
                }
            }

            if is_key_pressed_repeated(rl, Key::Left) && *cursor_index > 0 {
                self.cursor_tick = 0;
                let old_index = *cursor_index;

                let mut new_index = if ctrl {
                    floor_word_boundary(&self.input, *cursor_index)
                } else {
                    floor_char_boundary(&self.input, *cursor_index)
                };

                if shift {
                    if let Some((start, end)) = &mut self.select_range {
                        if *start == old_index { // started on the left, expand selection
                            *start = new_index;
                        } else if *end == old_index { // started on the right, retract selection
                            *end = new_index;
                            if *start == *end { // went back to the start, remove selection
                                self.select_range = None;
                            }
                        }
                    } else {
                        self.select_range = Some((new_index, old_index));
                    }
                } else if let Some((start, _)) = self.select_range {
                    self.select_range = None;
                    // put cursor to the left of selection (matches behaviour on web browsers)
                    new_index = start; 
                }

                *cursor_index = new_index;
            }
            if is_key_pressed_repeated(rl, Key::Right) && *cursor_index < self.input.len() {
                self.cursor_tick = 0;
                let old_index = *cursor_index;

                let mut new_index = if ctrl {
                    ceil_word_boundary(&self.input, *cursor_index)
                } else {
                    ceil_char_boundary(&self.input, *cursor_index)
                };

                if shift {
                    if let Some((start, end)) = &mut self.select_range {
                        if *start == old_index { // started on the left, retract selection
                            *start = new_index;
                            if *start == *end {  // went back to start, remove selection
                                self.select_range = None;
                            }
                        } else if *end == old_index { // started on the right, expand selection
                            *end = new_index;
                        }
                    } else {
                        self.select_range = Some((old_index, new_index));
                    }
                } else if let Some((_, end)) = self.select_range {
                    self.select_range = None;
                    // put cursor to the right when going out of selection (matches behaviour on web browsers)
                    new_index = end;
                }

                *cursor_index = new_index;
            }
            if is_key_pressed_repeated(rl, Key::Backspace) {
                if let Some((start, end)) = self.select_range { // remove selection
                    *cursor_index = start; // in case we expanded the selection to the right
                    self.input.drain(start..end);
                    self.select_range = None;
                } else if *cursor_index > 0 {
                    *cursor_index = floor_char_boundary(&self.input, *cursor_index);
                    self.input.remove(*cursor_index);
                }
                modified = true;
            }


            if modified {
                self.update_input(true);
            }

        } else {
            self.cursor_tick = 0;
        }

        if is_key_pressed(rl, Key::Escape) { quit(rl); }

        if is_key_pressed_repeated(rl, Key::Down) || (ctrl && is_key_pressed_repeated(rl, Key::J)) || (ctrl && is_key_pressed_repeated(rl, Key::N)) {
            // TODO: gently scroll window to selected choice
            self.selected += 1;
            self.selected = self.selected.min(self.entries.list.len().saturating_sub(1));
        }
        if is_key_pressed_repeated(rl, Key::Up) || (ctrl && is_key_pressed_repeated(rl, Key::K)) || (ctrl && is_key_pressed_repeated(rl, Key::P)) {
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
                Message::Entries(entries) => self.entries = Entries::new(entries, rl, &self.font),
                Message::Action(action) => return self.handle_action(rl, action),
            };
        }
    }
}

impl Keal {
    pub fn update_input(&mut self, from_user: bool) {

        match &mut self.cursor_index {
            Some(cursor_index) if from_user => *cursor_index = (*cursor_index).min(self.input.len()),
            cursor_index => *cursor_index = Some(self.input.len())
        }
        self.select_range = None;

        self.manager.send(async_manager::Event::UpdateInput(self.input.clone(), from_user));
    }

    fn handle_action(&mut self, rl: &mut Raylib, action: Action) /* -> Command<Message> */ {
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
            }
            Action::Exec(mut command) => {
                let _ = command.0.exec();
                quit(rl);
            }
            Action::PrintAndClose(message) => {
                println!("{message}");
                quit(rl);
            }
            Action::Fork => match fork().expect("failed to fork") {
                Fork::Parent(_) => quit(rl),
                Fork::Child => ()
            }
            Action::WaitAndClose => {
                self.manager.with_manager(|m| m.wait());
                quit(rl);
            }
        }
    }
}
