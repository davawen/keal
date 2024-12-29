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

pub struct TextInput {
    /// Modifying `input` should call [`Self::update_input`]
    pub text: String,

    font: FontFamily,
    /// Layout should be modified to reflect `text`
    layout: TextLayout,
    placeholder_layout: TextLayout,
    /// byte index of the cursor in the text input, None if the input is not selected
    cursor_index: Option<usize>,
    cursor_tick: usize,
    /// byte indices of the start and end ranges of the selection
    select_range: Option<(usize, usize)>,

    /// wether the mouse is hovering over the input
    hovered: bool,

    clipboard: ClipboardContext
}

impl TextInput {
    pub fn new(rc: &mut RenderContext, config: &Config, theme: &Theme, font: FontFamily) -> Self {
        let text = rc.text();
        let layout = text.new_text_layout("").build().unwrap();
        let placeholder_layout = text.new_text_layout(config.placeholder_text.clone())
            .font(font.clone(), config.font_size as f64 * 1.25)
            .text_color(theme.text)
            .default_attribute(FontWeight::MEDIUM)
            .build().unwrap();

        Self {
            text: String::new(),
            font,
            layout,
            placeholder_layout,
            cursor_index: Some(0),
            cursor_tick: 0,
            select_range: None,
            hovered: false,
            clipboard: ClipboardContext::new().unwrap()
        }
    }

    pub fn render(&mut self, rc: &mut RenderContext, config: &Config, theme: &Theme){
        let search_bar_height = (config.font_size as f64*3.25).ceil();

        let size = config.font_size as f64 * 1.25;

        let left_padding = config.font_size as f64;
        let baseline = (search_bar_height/2.0 - size/2.0).ceil();

        let screen_width = rc.target().width() as f64;

        rc.fill(kurbo::RoundedRect::new(0.0, 0.0, screen_width, search_bar_height, (5.0, 5.0, 0.0, 0.0)), &theme.input_background);

        let layout = if self.text.is_empty() && self.cursor_index.is_none() { &self.placeholder_layout } else { &self.layout };
        rc.draw_text(&layout, (left_padding, baseline));

        if let Some((start, end)) = self.select_range {
            let mut rect = layout.rects_for_range(start..end)[0];
            if end == self.text.len() {
                rect.x1 = layout.size().width;
            }
            rc.fill(rect.with_origin((rect.x0 + left_padding, rect.y0 + baseline)), &theme.input_selection);
        } else if let Some(cursor_index) = self.cursor_index {
            let cursor_position = if self.text.is_empty() {
                0.0
            } else if cursor_index == self.text.len() {
                layout.size().width
            } else {
                layout.rects_for_range(cursor_index..cursor_index+1)[0].x0
            };

            let pos = left_padding + cursor_position;
            rc.stroke(kurbo::Line::new((pos, baseline), Point::new(pos, baseline + size + 5.0)), &Color::WHITE, 1.0);
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
            self.cursor_index = Some(hit.idx);
        }
    }

    /// Returns whether the input was modified
    /// 
    /// If this function returns true, the calling function should ensure [`Self::update_input`] is called.
    pub fn on_key_press(&mut self, key: &KeyEvent, ui_state: &crate::UiState) -> bool {
        let ctrl = ui_state.ctrl;
        let shift = ui_state.shift;

        if let Some(cursor_index) = &mut self.cursor_index {
            let mut modified = false;

            if ctrl {
                match key.physical_key {
                    PhysicalKey::Code(KeyCode::KeyA) => self.select_range = Some((0, self.text.len())),
                    PhysicalKey::Code(KeyCode::KeyC) => {
                        if let Some((start, end)) = self.select_range {
                            let text = &self.text[start..end];
                            self.clipboard.set_contents(text.to_owned()).unwrap();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyX) => {
                        if let Some((start, end)) = self.select_range {
                            *cursor_index = start; // in case we expanded the selection to the right
                            self.select_range = None;

                            let text = self.text.drain(start..end).collect::<String>();
                            self.clipboard.set_contents(text).unwrap();
                            modified = true;
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyV) => {
                        if let Some((start, end)) = self.select_range {
                            *cursor_index = start; // in case we expanded the selection to the right
                            self.text.drain(start..end);
                            self.select_range = None;
                            modified = true;
                        }

                        match self.clipboard.get_contents() {
                            Ok(text) if !text.is_empty() => {
                                self.text.insert_str(*cursor_index, &text);
                                *cursor_index += text.len();
                                modified = true;
                            }
                            _ => (),
                        }
                    }
                    _ => ()
                }
            } else if let (PhysicalKey::Code(KeyCode::ArrowLeft), true) = (key.physical_key, *cursor_index > 0) {
                self.cursor_tick = 0;
                let old_index = *cursor_index;

                let mut new_index = if ctrl {
                    floor_word_boundary(&self.text, *cursor_index)
                } else {
                    floor_char_boundary(&self.text, *cursor_index)
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
            } else if let (PhysicalKey::Code(KeyCode::ArrowRight), true) = (key.physical_key, *cursor_index < self.text.len()) {
                self.cursor_tick = 0;
                let old_index = *cursor_index;

                let mut new_index = if ctrl {
                    ceil_word_boundary(&self.text, *cursor_index)
                } else {
                    ceil_char_boundary(&self.text, *cursor_index)
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
            } else if let PhysicalKey::Code(KeyCode::Backspace) = key.physical_key {
                if let Some((start, end)) = self.select_range { // remove selection
                    *cursor_index = start; // in case we expanded the selection to the right
                    self.text.drain(start..end);
                    self.select_range = None;
                } else if *cursor_index > 0 {
                    *cursor_index = floor_char_boundary(&self.text, *cursor_index);
                    self.text.remove(*cursor_index);
                }
                modified = true;
            } else if let Some(text) = &key.text {
                if !text.contains(|c: char| c == '\n' || c == '\r' || c.is_control()) {
                    if let Some((start, end)) = self.select_range { // remove selected text
                        *cursor_index = start;
                        self.text.drain(start..end);
                        self.select_range = None;
                    }

                    self.text.insert_str(*cursor_index, text.as_str());
                    *cursor_index += text.len();

                    self.cursor_tick = 0;
                    modified = true;
                }
            }

            modified
        } else {
            self.cursor_tick = 0;
            false
        }
    }

    pub fn update_input(&mut self, rc: &mut RenderContext, config: &Config, theme: &Theme, from_user: bool) {
        match &mut self.cursor_index {
            Some(cursor_index) if from_user => *cursor_index = (*cursor_index).min(self.text.len()),
            cursor_index => *cursor_index = Some(self.text.len())
        }
        self.select_range = None;

        let rc_text = rc.text();
        let layout = rc_text.new_text_layout(self.text.clone())
            .font(self.font.clone(), pixels_to_pts(config.font_size as f64 * 1.25))
            .text_color(theme.text)
            .default_attribute(FontWeight::MEDIUM)
            .build().unwrap();

        self.layout = layout;
    }
}
