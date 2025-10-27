use super::{pixels_to_pts, RenderContext, RenderContextTrait, TextLayout, TextLayoutBuilderTrait, TextLayoutTrait, TextTrait};
use piet_tiny_skia::piet::{kurbo::{self, Point}, Color, FontFamily, FontWeight};

use keal::config::Config;
use winit::{dpi::PhysicalPosition, event::KeyEvent, keyboard::{KeyCode, PhysicalKey}, window::Window};

use copypasta::{ClipboardContext, ClipboardProvider};

use crate::config::Theme;

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

type StrPosFn = fn(&str, usize) -> usize;

enum Selection {
    None,
    Cursor(usize),
    Select { pivot: usize, cursor: usize }
}


impl Selection {
    fn is_none(&self) -> bool { matches!(self, Selection::None) }
    fn select_range(&self) -> Option<(usize, usize)> {
        match self {
            &Self::Select { pivot, cursor } if pivot < cursor => Some((pivot, cursor)),
            &Self::Select { pivot, cursor } => Some((cursor, pivot)),
            _ => None
        }
    }
}


pub struct TextInput {
    /// Modifying `input` should call [`Self::update_input`]
    pub text: String,

    font: FontFamily,
    /// Layout should be modified to reflect `text`
    layout: TextLayout,
    placeholder_layout: TextLayout,
    cursor_tick: usize,

    /// current cursor position or selection state
    selection: Selection,

    /// wether the mouse is hovering over the input
    hovered: bool,

    clipboard: ClipboardContext
}

impl TextInput {
    pub fn new(rc: &mut RenderContext, config: &Config, theme: &Theme, font: FontFamily) -> Self {
        let text = rc.text();
        let layout = text.new_text_layout("").build().unwrap();
        let placeholder_layout = text.new_text_layout(config.placeholder_text.clone())
            .font(font.clone(), (config.font_size as f64 * 1.25).ceil())
            .text_color(theme.text)
            .default_attribute(FontWeight::MEDIUM)
            .build().unwrap();

        Self {
            text: String::new(),
            font,
            layout,
            placeholder_layout,
            cursor_tick: 0,
            selection: Selection::None,
            hovered: false,
            clipboard: ClipboardContext::new().unwrap()
        }
    }

    pub fn render(&mut self, rc: &mut RenderContext, config: &Config, theme: &Theme){
        let search_bar_height = (config.font_size as f64*3.25).ceil();

        let size = (config.font_size as f64 * 1.25).ceil();

        let left_padding = (config.font_size as f64).ceil();
        let baseline = (search_bar_height/2.0 - size/2.0).ceil();

        let screen_width = rc.target().width() as f64;

        rc.fill(kurbo::RoundedRect::new(0.0, 0.0, screen_width, search_bar_height, (5.0, 5.0, 0.0, 0.0)), &theme.input_background);

        let layout = if self.text.is_empty() && self.selection.is_none() { &self.placeholder_layout } else { &self.layout };

        let f = layout.line_metric(0).unwrap_or_default().baseline.fract();
        rc.draw_text(&layout, (left_padding, baseline + f));

        if let Some((start, end)) = self.selection.select_range() {
            let mut rect = layout.rects_for_range(start..end)[0];
            if end == self.text.len() {
                rect.x1 = layout.size().width;
            }
            rc.fill(rect.with_origin((rect.x0 + left_padding, rect.y0 + baseline)), &theme.input_selection);
        } else if let Selection::Cursor(cursor) = self.selection {
            let cursor_position = if self.text.is_empty() {
                0.0
            } else if cursor == self.text.len() {
                layout.size().width
            } else {
                layout.rects_for_range(cursor..cursor+1)[0].x0
            };

            let pos = (left_padding + cursor_position).ceil();
            rc.stroke(kurbo::Line::new((pos + 0.5, baseline), Point::new(pos + 0.5, (baseline + size + 5.0).round())), &Color::WHITE, 1.0);
        }
    }

    pub fn on_cursor_moved(&mut self, config: &Config, window: &Window, PhysicalPosition { x: _, y }: PhysicalPosition<f64>) {
        let search_bar_height = (config.font_size as f64*3.25).ceil();
        self.hovered = y >= 0.0 && y < search_bar_height;

        if self.hovered {
            window.set_cursor(winit::window::CursorIcon::Text);
        } else {
            window.set_cursor(winit::window::CursorIcon::Default);
        }
    }

    pub fn on_left_click(&mut self, config: &Config, ui_state: &crate::UiState) {
        let left_padding = config.font_size as f64;
        if self.hovered {
            let hit = self.layout.hit_test_point((ui_state.mouse_pos.x - left_padding, 0.0).into());
            self.selection = Selection::Cursor(hit.idx);
        }
    }

    fn move_cursor(&mut self, char_call: StrPosFn, word_call: StrPosFn, right: bool, ctrl: bool, shift: bool) {
        let call = if ctrl { word_call } else { char_call };

        let (old_cursor, new_cursor) = match self.selection {
            Selection::None => return,
            // When leaving selection, choose the selection bound matching the direction of the key pressed
            Selection::Select { pivot, cursor } if !shift => {
                let bound = if (right && pivot < cursor) || (!right && cursor < pivot) { cursor }
                else { pivot };

                if ctrl { (bound, (word_call)(&self.text, bound)) } else { (bound, bound) }
            }
            Selection::Cursor(cursor) | Selection::Select { pivot: _, cursor } => (cursor, (call)(&self.text, cursor)),
        };

        if shift {
            if let Selection::Select { pivot, cursor } = &mut self.selection {
                if *pivot == new_cursor { self.selection = Selection::Cursor(new_cursor) }
                else { *cursor = new_cursor; }
            } else if new_cursor != old_cursor {
                self.selection = Selection::Select { cursor: new_cursor, pivot: old_cursor };
            } else {
                self.selection = Selection::Cursor(new_cursor);
            }
        } else {
            self.selection = Selection::Cursor(new_cursor);
        }
    }

    /// Returns whether the input was modified
    /// 
    /// If this function returns true, the calling function should ensure [`Self::update_input`] is called.
    pub fn on_key_press(&mut self, key: &KeyEvent, ui_state: &crate::UiState) -> bool {
        let ctrl = ui_state.ctrl;
        let shift = ui_state.shift;

        if !matches!(self.selection, Selection::None) {
            let mut modified = false;

            match key.physical_key {
                PhysicalKey::Code(KeyCode::KeyA) if ctrl => self.selection = Selection::Select { pivot: 0, cursor: self.text.len() },
                PhysicalKey::Code(KeyCode::KeyC) if ctrl => {
                    if let Some((start, end)) = self.selection.select_range() {
                        let text = &self.text[start..end];
                        self.clipboard.set_contents(text.to_owned()).unwrap();
                    }
                }
                PhysicalKey::Code(KeyCode::KeyX) if ctrl => {
                    if let Some((start, end)) = self.selection.select_range() {
                        self.selection = Selection::Cursor(start);

                        let text = self.text.drain(start..end).collect::<String>();
                        self.clipboard.set_contents(text).unwrap();
                        modified = true;
                    }
                }
                PhysicalKey::Code(KeyCode::KeyV) if ctrl => {
                    let cursor = if let Some((start, end)) = self.selection.select_range() {
                        self.text.drain(start..end);
                        modified = true;
                        start
                    } else if let Selection::Cursor(cursor) = self.selection { cursor }
                    else { unreachable!() };

                    match self.clipboard.get_contents() {
                        Ok(text) if !text.is_empty() => {
                            self.text.insert_str(cursor, &text);
                            self.selection = Selection::Cursor(cursor + text.len());
                            modified = true;
                        }
                        _ => (),
                    }
                }
                PhysicalKey::Code(KeyCode::ArrowLeft) => {
                    self.cursor_tick = 0;
                    self.move_cursor(floor_char_boundary, floor_word_boundary, false, ctrl, shift);
                }
                PhysicalKey::Code(KeyCode::ArrowRight) => {
                    self.cursor_tick = 0;
                    self.move_cursor(ceil_char_boundary, ceil_word_boundary, true, ctrl, shift);
                }
                PhysicalKey::Code(KeyCode::Backspace) => {
                    if let Some((start, end)) = self.selection.select_range() { // remove selection
                        self.text.drain(start..end);
                        self.selection = Selection::Cursor(start);
                    } else if let Selection::Cursor(cursor) = &mut self.selection && *cursor > 0 {
                        *cursor = floor_char_boundary(&self.text, *cursor);
                        self.text.remove(*cursor);
                    }
                    modified = true;
                }
                PhysicalKey::Code(KeyCode::Delete) => {
                    if let Some((start, end)) = self.selection.select_range() { // remove selection
                        self.text.drain(start..end);
                        self.selection = Selection::Cursor(start);
                    } else if let Selection::Cursor(cursor) = &mut self.selection && *cursor < self.text.len() {
                        self.text.remove(*cursor);
                    }
                    modified = true;
                }
                _ => if let Some(text) = &key.text {
                    if !text.contains(|c: char| c == '\n' || c == '\r' || c.is_control()) {
                        if let Some((start, end)) = self.selection.select_range() { // remove selected text
                            self.text.drain(start..end);
                            self.selection = Selection::Cursor(start);
                        }

                        if let Selection::Cursor(cursor) = &mut self.selection {
                            self.text.insert_str(*cursor, text.as_str());
                            *cursor += text.len();

                            self.cursor_tick = 0;
                            modified = true;
                        }
                    }
                }
            }

            modified
        } else {
            self.cursor_tick = 0;
            false
        }
    }

    pub fn update_input(&mut self, rc: &mut RenderContext, config: &Config, theme: &Theme, from_user: bool) {
        match &mut self.selection {
            Selection::Cursor(cursor) if from_user => *cursor = (*cursor).min(self.text.len()),
            selection => *selection = Selection::Cursor(self.text.len())
        }

        let rc_text = rc.text();
        let layout = rc_text.new_text_layout(self.text.clone())
            .font(self.font.clone(), pixels_to_pts(config.font_size as f64 * 1.25))
            .text_color(theme.text)
            .default_attribute(FontWeight::REGULAR)
            .build().unwrap();

        self.layout = layout;
    }
}
