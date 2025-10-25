use std::{sync::{mpsc::{Receiver, Sender, TryRecvError}, Arc, OnceLock}};

use raylib::prelude::*;
use smallvec::SmallVec;

use keal::{config::config, icon::{Icon, IconCache, IconPath}, log_time, plugin::{entry::DisplayEntry, FrontendAction, FrontendEvent}};
use text_input::TextInput;
use crate::config::Theme;

mod text_input;

pub type TTFCache = TrueTypeFontCache;

fn is_key_pressed_repeated(rl: &mut Raylib, key: Key) -> bool {
    is_key_pressed(rl, key) || is_key_pressed_again(rl, key)
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
    list: Vec<DisplayEntry>,
    /// info for entry.name and entry.comment (optional)
    wrap_info: Vec<(WrapInfo, Option<WrapInfo>)>,
    total_height: f32,
    calculated: bool
}

impl Entries {
    fn new(list: Vec<DisplayEntry>, rl: &mut Raylib, atlas: &TTFCache) -> Self {
        let mut this = Self {
            list,
            wrap_info: Vec::new(),
            total_height: 0.0,
            calculated: false
        }
    }

    /// call this when the screen width changes
    fn recalculate(&mut self, rl: &mut Raylib, font: &TTFCache) {
        let config = config();

        self.total_height = 0.0;
        self.wrap_info.clear();
        self.wrap_info.extend(self.list.iter().map(|entry| {
            let icon_width = entry.icon.as_ref().map(|_| config.font_size + 4.0).unwrap_or_default();

            let name = measure_text_wrap(&entry.name.source(), get_screen_width(rl)/2.0 - icon_width, font, config.font_size, 5.0);
            let mut max_height = name.height;

            let comment_width = get_screen_width(rl) - name.width - icon_width - 10.0 - 20.0 - 10.0; // this removes: name left padding, name-comment inner padding, comment right padding
            let comment = entry.comment.as_ref()
                .map(|comment| measure_text_wrap(comment.source(), comment_width, font, config.font_size, 5.0))
                .inspect(|comment| max_height = max_height.max(comment.height));

            self.total_height += max_height + 20.0;

            (name, comment)
        }));

        self.calculated = true;
    }
}

pub struct Keal {
    // -- UI state --
    input: text_input::TextInput,

    scroll: f32,

    selected: usize,
    hovered_choice: Option<usize>,

    old_screen_width: f32,

    rendered_icons: std::collections::HashMap<IconPath, Option<Texture>>,

    // -- Data state --
    icons: Arc<OnceLock<IconCache>>,
    font: TrueTypeFontCache,

    entries: Entries,

    action_rec: Receiver<FrontendAction>,
    event_sender: Sender<FrontendEvent>
}

impl Keal {
    pub fn new() -> Self {
        log_time("initializing app");

        let config = config();

        let icons = Arc::new(OnceLock::new());

        {
            let icons = icons.clone();
            std::thread::spawn(move || {
                let icon_cache = IconCache::new(&config.icon_theme);
                icons.set(icon_cache).expect("Nothing should have set icon cache before this");
            });
        }

        let (event_sender, action_rec) = keal::plugin::init(50, true);

        log_time("finished initializing");

        Keal {
            input: TextInput::default(),
            scroll: 0.0,
            selected: 0,
            hovered_choice: None,
            old_screen_width: 0.0,
            rendered_icons: Default::default(),
            icons,
            font,
            entries: Default::default(),
            event_sender,
            action_rec
        }
    }

    pub fn render(&mut self, rl: &mut DrawHandle, theme: &Theme) {
        let entries = &self.entries;
        let config = config();

        let Some(font) = &self.font else { return };
        let font_size = config.font_size;

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

            let mut rectangle_color = theme.choice_background;
            if mouse.y >= offset_y && mouse.y < next_offset_y {
                self.hovered_choice = Some(index);
                rectangle_color = theme.hovered_choice_background;
            }
            if selected { rectangle_color = theme.selected_choice_background; } 

            draw_rectangle(rl, 0.0, offset_y, get_screen_width(rl), next_offset_y-offset_y, rectangle_color);

            let mut icon_offset = 10.0;

            if let Some(icon_path) = &entry.icon {
                if let Some(rendered) = self.rendered_icons.get(icon_path) {
                    if let Some(rendered) = rendered {
                        draw_texture_ex(rl, rendered, vec2(icon_offset, offset_y + 10.0), 0.0, config.font_size / rendered.width() as f32, Color::WHITE);
                        icon_offset += config.font_size + 4.0;
                    }
                } else if let Some(icon_cache) = self.icons.get() && let Some(icon) = icon_cache.get(icon_path) {
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
                let mut offset = icon_offset;
                for ((a, b), highlighted) in entry.name.iter_indices() {
                    if b < line_start { continue }
                    if a > line_end { break }

                    let a = a.clamp(line_start, line_end);
                    let b = b.clamp(line_start, line_end);

                    let span = &entry.name.source()[a..b];

                    let color = match highlighted {
                        false => theme.text,
                        true => match selected {
                            false => theme.matched_text,
                            true => theme.selected_matched_text
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
                    let text = &comment.source()[line_start..line_end];

                    draw_text(rl, font, text, vec2(get_screen_width(rl) - wrap_info.width - 10.0, comment_offset_y), font_size, theme.comment);
                    comment_offset_y += config.font_size + 5.0;
                    line_start = line_end;
                }
            }

            offset_y = next_offset_y;
        }

        self.input.render(rl, font, config, theme);
    }

    pub fn update(&mut self, rl: &mut Raylib) {
        if let Some(font) = &self.font {
            if self.entries.calculated == false || self.old_screen_width != get_screen_width(rl) {
                self.entries.recalculate(rl, font);
                self.old_screen_width = get_screen_width(rl);
            }
        }

        if let Some(hovered_choice) = self.hovered_choice {
            set_mouse_cursor(rl, MouseCursor::PointingHand);

            if is_mouse_button_pressed(rl, MouseButton::Left) {
                let selected = Some(self.entries.list[hovered_choice].label);
                let _ = self.event_sender.send(FrontendEvent::Launch(selected));
            }
        } 

        if self.input.update(rl) {
            self.update_input(true);
        }

        if is_key_pressed(rl, Key::Enter) {
            let selected = self.entries.list.get(self.selected).map(|x| x.label);
            let _ = self.event_sender.send(FrontendEvent::Launch(selected));
        }

        if is_key_pressed(rl, Key::Escape) { quit(rl); }

        // TODO: Refactor
        let snap_selected_to_edge = |rl: &mut Raylib, this: &mut Keal| { // returns the
            let search_bar_height = (config().font_size*3.25).ceil();
            let mut offset_y = 0.0;
            for (index, wrap_info) in this.entries.wrap_info.iter().enumerate() {
                let max_height = wrap_info.0.height.max(wrap_info.1.as_ref().map(|x| x.height).unwrap_or(0.0));

                if index == this.selected {
                    this.scroll = this.scroll.clamp(
                        offset_y - get_render_height(rl) + search_bar_height + max_height + 20.0,
                        offset_y
                    );
                    break;
                }

                offset_y += max_height + 20.0;
            }
        };

        let ctrl = is_key_down(rl, Key::LeftControl) || is_key_down(rl, Key::RightControl);

        if is_key_pressed_repeated(rl, Key::Down) || (ctrl && is_key_pressed_repeated(rl, Key::J)) || (ctrl && is_key_pressed_repeated(rl, Key::N)) {
            self.selected += 1;
            self.selected = self.selected.min(self.entries.list.len().saturating_sub(1));
            snap_selected_to_edge(rl, self);
        }
        if is_key_pressed_repeated(rl, Key::Up) || (ctrl && is_key_pressed_repeated(rl, Key::K)) || (ctrl && is_key_pressed_repeated(rl, Key::P)) {
            self.selected = self.selected.saturating_sub(1);
            snap_selected_to_edge(rl, self);
        }

        loop {
            match self.action_rec.try_recv() {
                Ok(action) => self.handle_action(rl, action),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("manager channel disconnected")
            }
        }
    }
}

impl Keal {
    pub fn update_input(&mut self, from_user: bool) {
        self.input.update_input(from_user);
        let _ = self.event_sender.send(FrontendEvent::UpdateInput { input: self.input.text.clone(), from_user });
    }

    fn handle_action(&mut self, rl: &mut Raylib, action: FrontendAction) /* -> Command<Message> */ {
        match action {
            FrontendAction::ChangeInput(new) => {
                self.input.text = new;
                self.update_input(false);
            }
            FrontendAction::UpdateEntries { entries, query: _ } => {
                self.entries = Entries::new(entries, rl, &self.font);
            }
            FrontendAction::Close => quit(rl)
        }
    }
}
