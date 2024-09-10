use std::ffi::{CStr, CString};

use raylib::prelude::*;

use keal::config::Config;

use crate::config::Theme;

use super::{draw_rectangle_rounded, is_key_pressed_repeated, TTFCache};

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
    /// byte index of the cursor in the text input, None if the input is not selected
    cursor_index: Option<usize>,
    cursor_tick: usize,
    /// byte indices of the start and end ranges of the selection
    select_range: Option<(usize, usize)>,

    /// wether the mouse is hovering over the input
    hovered: bool
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            text: String::new(),
            cursor_index: Some(0),
            cursor_tick: 0,
            select_range: None,
            hovered: false
        }
    }
}

impl TextInput {
    pub fn render(&mut self, rl: &mut DrawHandle, font: &TTFCache, config: &Config, theme: &Theme){
        let search_bar_height = (config.font_size*3.25).ceil();

        let text = if self.text.is_empty() && self.cursor_index.is_none() { &config.placeholder_text } else { &self.text };

        let size = config.font_size*1.25;

        let left_padding = config.font_size;
        let baseline = (search_bar_height/2.0 - size/2.0).ceil();

        draw_rectangle_rounded(rl, 0.0, 0.0, get_screen_width(rl), search_bar_height, [5.0, 5.0, 0.0, 0.0], theme.input_background);
        draw_text(rl, font, &text, vec2(left_padding, baseline), size, theme.text);

        if let Some((start, end)) = self.select_range {
            let start_pos = if self.text.is_empty() { 0.0 } else { measure_text(font, &text[0..start], size).x };
            let end_pos = if self.text.is_empty() { 0.0 } else { measure_text(font, &text[0..end], size).x };
            draw_rectangle(rl, left_padding + start_pos - 1.0, baseline, end_pos - start_pos + 2.0, size + 5.0, theme.input_selection);
        } else if let Some(cursor_index) = self.cursor_index {
            let cursor_position = if self.text.is_empty() { 0.0 } else { measure_text(font, &text[0..cursor_index], size).x };

            if self.cursor_tick % 60 < 30 {
                draw_rectangle(rl, left_padding + cursor_position - 1.0, baseline, 1.0, size + 5.0, Color::WHITE);
            }
        }

        let mouse = get_mouse_pos(rl);
        self.hovered = mouse.y >= 0.0 && mouse.y < search_bar_height;
    }

    /// Returns whether the input was modified
    /// 
    /// If this function returns true, the calling function should call [`Self::update_input`] in some way or another.
    pub fn update(&mut self, rl: &mut Raylib) -> bool {
        if self.hovered {
            set_mouse_cursor(rl, MouseCursor::Ibeam);

            if is_mouse_button_pressed(rl, MouseButton::Left) {
                self.cursor_index = Some(0);
            }
        } else {
            set_mouse_cursor(rl, MouseCursor::Default);
        }

        let ctrl = is_key_down(rl, Key::LeftControl) || is_key_down(rl, Key::RightControl);
        let shift = is_key_down(rl, Key::LeftShift) || is_key_down(rl, Key::RightShift);

        if let Some(cursor_index) = &mut self.cursor_index {
            self.cursor_tick += 1;

            let mut modified = false;
            while let Some(ch) = get_char_pressed(rl) {
                if let Some((start, end)) = self.select_range { // remove selected text
                    *cursor_index = start;
                    self.text.drain(start..end);
                    self.select_range = None;
                }

                self.text.insert(*cursor_index, ch);
                *cursor_index += ch.len_utf8();

                self.cursor_tick = 0;
                modified = true;
            }

            if ctrl {
                if is_key_pressed(rl, Key::A) {
                    self.select_range = Some((0, self.text.len()));
                }
                if is_key_pressed(rl, Key::C) {
                    if let Some((start, end)) = self.select_range {
                        let text = &self.text[start..end];
                        set_clipboard_text(rl, &CString::new(text).unwrap());
                    }
                }
                if is_key_pressed(rl, Key::X) {
                    if let Some((start, end)) = self.select_range {
                        *cursor_index = start; // in case we expanded the selection to the right
                        self.select_range = None;

                        let mut text = self.text.drain(start..end).collect::<String>().into_bytes();
                        text.push(0);
                        set_clipboard_text(rl, CStr::from_bytes_until_nul(&text).unwrap());
                        modified = true;
                    }
                }
                if is_key_pressed(rl, Key::V) {
                    if let Some((start, end)) = self.select_range {
                        *cursor_index = start; // in case we expanded the selection to the right
                        self.text.drain(start..end);
                        self.select_range = None;
                        modified = true;
                    }

                    match get_clipboard_text(rl).to_str() {
                        Ok(text) if !text.is_empty() => {
                            self.text.insert_str(*cursor_index, text);
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
            }
            if is_key_pressed_repeated(rl, Key::Right) && *cursor_index < self.text.len() {
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
            }
            if is_key_pressed_repeated(rl, Key::Backspace) {
                if let Some((start, end)) = self.select_range { // remove selection
                    *cursor_index = start; // in case we expanded the selection to the right
                    self.text.drain(start..end);
                    self.select_range = None;
                } else if *cursor_index > 0 {
                    *cursor_index = floor_char_boundary(&self.text, *cursor_index);
                    self.text.remove(*cursor_index);
                }
                modified = true;
            }

            modified
        } else {
            self.cursor_tick = 0;
            false
        }
    }

    pub fn update_input(&mut self, from_user: bool) {
        match &mut self.cursor_index {
            Some(cursor_index) if from_user => *cursor_index = (*cursor_index).min(self.text.len()),
            cursor_index => *cursor_index = Some(self.text.len())
        }
        self.select_range = None;
    }
}
