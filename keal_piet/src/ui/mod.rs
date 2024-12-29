use std::{os::unix::process::CommandExt, sync::mpsc::{channel, Receiver, Sender, TryRecvError}};

use async_manager::Data;
use fork::{fork, Fork};
use nucleo_matcher::Matcher;

use keal::{config::{config, Config}, icon::{Icon, IconCache, IconPath}, log_time, plugin::{entry::{Label, OwnedEntry}, Action}};
use resvg::{tiny_skia::{FilterQuality, Pixmap, PixmapPaint}, usvg::{Size, Transform}};
use text_input::TextInput;
use winit::{dpi::PhysicalPosition, event::KeyEvent, keyboard::{KeyCode, PhysicalKey}, window::{CursorIcon, Window}};
use crate::config::Theme;

use self::{match_span::MatchSpan, async_manager::AsyncManager};

use piet_tiny_skia::{self as pts, piet::TextAttribute, AsPixmapMut};
use pts::{TextLayout, piet::{kurbo, FontFamily, Text as TextTrait, TextLayout as TextLayoutTrait, TextLayoutBuilder as TextLayoutBuilderTrait, RenderContext as RenderContextTrait}};

pub type RenderContext<'a> = pts::RenderContext<'a, pts::tiny_skia::PixmapMut<'a>>;

mod match_span;
mod async_manager;

mod text_input;

pub fn pixels_to_pts(pixel: f64) -> f64 {
    (pixel * 72.0 / 96.0).ceil()
}

struct CachedLayout {
    name: TextLayout,
    name_selected: TextLayout,
    comment: Option<TextLayout>
}

impl CachedLayout {
    fn max_height(&self) -> f64 {
        self.name.size().height.max(self.comment.as_ref().map(|x| x.size().height).unwrap_or(0.0))
    }
}

#[derive(Default)]
struct Entries {
    list: Vec<OwnedEntry>,
    /// info for entry.name and entry.comment (optional)
    wrap_info: Vec<CachedLayout>,
    total_height: f64
}

impl Entries {
    fn new(list: Vec<OwnedEntry>, rc: &mut RenderContext, theme: &Theme, font: &FontFamily, data: &mut Data) -> Self {
        let mut this = Self {
            list,
            wrap_info: Vec::new(),
            total_height: 0.0
        };

        this.recalculate(rc, theme, font, data);
        this
    }

    /// call this when the screen width changes
    fn recalculate(&mut self, rc: &mut RenderContext, theme: &Theme, font: &FontFamily, data: &mut Data) {
        let config = config();

        let mut buf = vec![];

        self.total_height = 0.0;
        self.wrap_info.clear();
        self.wrap_info.extend(self.list.iter().map(|entry| {
            let icon_width = entry.icon.as_ref().map(|_| config.font_size as f64 + 4.0).unwrap_or_default();

            let screen_width = rc.target().width() as f64;

            let text = rc.text();

            let mut name = text.new_text_layout(entry.name.clone())
                .max_width(screen_width/2.0 - icon_width)
                .font(font.clone(), pixels_to_pts(config.font_size as f64));
            
            let mut name_selected = text.new_text_layout(entry.name.clone())
                .max_width(screen_width/2.0 - icon_width)
                .font(font.clone(), pixels_to_pts(config.font_size as f64));

            for (span, highlighted) in MatchSpan::new(&entry.name, &mut data.matcher, &data.pattern, &mut buf) {
                let (color, color_selected) = match highlighted {
                    false => (theme.text, theme.text),
                    true => (theme.matched_text, theme.selected_matched_text)
                };

                name = name.range_attribute(span.clone(), TextAttribute::TextColor(color));
                name_selected = name_selected.range_attribute(span, TextAttribute::TextColor(color_selected));
            }

            let name = name.build().unwrap();
            let name_selected = name_selected.build().unwrap();

            let name_size = name.size();

            let comment_width = screen_width - name_size.width - icon_width - 10.0 - 20.0 - 10.0; // this removes: name left padding, name-comment inner padding, comment right padding
            let comment = entry.comment.as_ref()
                .map(|comment| text.new_text_layout(comment.clone())
                    .max_width(comment_width)
                    .font(font.clone(), pixels_to_pts(config.font_size as f64))
                    .text_color(theme.comment)
                    .build().unwrap());
            
            let layout = CachedLayout { name, name_selected, comment };

            self.total_height += layout.max_height() + 26.0;

            layout
        }));
    }
}

pub struct Keal {
    // -- UI state --
    input: text_input::TextInput,

    scroll: f64,

    selected: usize,
    hovered_choice: Option<usize>,

    theme: &'static Theme,

    rendered_icons: std::collections::HashMap<IconPath, Option<Pixmap>>,

    pub quit: bool,

    // -- Data state --
    icons: IconCache,
    font: FontFamily,

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
    pub fn new(rc: &mut RenderContext, font: FontFamily, theme: &'static Theme) -> Self {
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

        let mut this = Keal {
            input: TextInput::new(rc, config, theme, font.clone()),
            scroll: 0.0,
            selected: 0,
            hovered_choice: None,
            rendered_icons: Default::default(),
            quit: false,
            theme,
            icons: Default::default(),
            font,
            entries: Default::default(),
            manager,
            message_sender,
            message_rec
        };
        this.update_input(rc, config, false);
        this
    }

    pub fn render(&mut self, ui_state: &super::UiState, rc: &mut RenderContext) {
        let entries = &self.entries;
        let theme = &self.theme;
        let config = config();

        // TODO: scrollbar

        let search_bar_height = (config.font_size as f64 * 3.25).ceil();
        let mouse = ui_state.mouse_pos;

        self.hovered_choice = None;

        let mut offset_y = search_bar_height - self.scroll;

        for (index, (entry, wrap_info)) in entries.list.iter().zip(entries.wrap_info.iter()).enumerate() {

            let max_height = wrap_info.max_height();
            let next_offset_y = offset_y + max_height + 26.0;

            if next_offset_y < search_bar_height { 
                offset_y = next_offset_y;
                continue
            }
            if offset_y > ui_state.screen_height { break }

            let selected = self.selected == index;

            let mut rectangle_color = theme.choice_background;
            if mouse.y >= offset_y && mouse.y < next_offset_y {
                self.hovered_choice = Some(index);
                rectangle_color = theme.hovered_choice_background;
            }
            if selected { rectangle_color = theme.selected_choice_background; } 

            rc.fill(kurbo::Rect::new(0.0, offset_y, ui_state.screen_width, next_offset_y), &rectangle_color);

            let mut icon_offset = 10.0;

            if let Some(icon_path) = &entry.icon {
                let mut draw_rendered = |rendered: &Pixmap| {
                        let scale = config.font_size / rendered.width() as f32;
                        let target = rc.target_mut();
                        target.draw_pixmap(
                            0, 0, rendered.as_ref(),
                            &PixmapPaint { quality: FilterQuality::Bilinear, ..Default::default() },
                            Transform::from_scale(scale, scale).post_concat(Transform::from_translate(icon_offset as f32, offset_y as f32 + 13.0)), None
                        );
                        icon_offset += config.font_size as f64 + 4.0;
                };

                match self.rendered_icons.get(icon_path) {
                    Some(Some(rendered)) => {
                        draw_rendered(&rendered);
                    }
                    Some(None) => (),
                    None => if let Some(icon) = self.icons.get(icon_path) {
                        match icon {
                            Icon::Svg(path) => {
                                let path = path.clone();
                                if let Ok(data) = std::fs::read(path) {
                                        // let _ = message_sender.send(Message::RenderedIcon(RenderedIcon::Failed));

                                    if let Ok(tree) = resvg::usvg::Tree::from_data(
                                        &data,
                                        &resvg::usvg::Options { default_size: Size::from_wh(config.font_size, config.font_size).unwrap(), ..Default::default() }
                                    ) {
                                        let size = tree.size();
                                        let mut pixmap = Pixmap::new(size.width() as u32, size.height() as u32).unwrap();
                                        resvg::render(&tree, Default::default(), &mut pixmap.as_pixmap_mut());
                                        draw_rendered(&pixmap);
                                        self.rendered_icons.insert(icon_path.clone(), Some(pixmap));
                                    } else {
                                        self.rendered_icons.insert(icon_path.clone(), None);
                                    };
                                } else {
                                    self.rendered_icons.insert(icon_path.clone(), None);
                                }
                            } 
                            Icon::Other(_path) => {
                                // TODO: Other icons
                                self.rendered_icons.insert(icon_path.clone(), None);
                            }
                        };
                    }
                }
            }

            let name = if selected { &wrap_info.name_selected } else { &wrap_info.name };
            rc.draw_text(name, (icon_offset, offset_y + 13.0));

            if let Some(comment) = &wrap_info.comment {
                rc.draw_text(comment, (ui_state.screen_width - comment.size().width - 10.0, offset_y + 13.0));
            }

            offset_y = next_offset_y;
        }

        self.input.render(rc, config, theme);
    }

    /// Call this on the event [`WindowEvent::Resized`]
    pub fn on_resize(&mut self, rc: &mut RenderContext) {
        let data = &mut *self.manager.get_data();
        self.entries.recalculate(rc, self.theme, &self.font, data);
    }

    /// Call this on the event [`WindowEvent::KeyboardInput`]
    pub fn on_key_press(&mut self, rc: &mut RenderContext, window: &Window, ui_state: &crate::UiState, key: KeyEvent) {
        window.request_redraw();

        let config = config();
        if self.input.on_key_press(&key, ui_state) {
            self.update_input(rc, config, true);
        }

        // TODO: Refactor
        let snap_selected_to_edge = |this: &mut Keal| { // returns the
            let search_bar_height = (config.font_size as f64 * 3.25).ceil();
            let mut offset_y = 0.0;
            for (index, wrap_info) in this.entries.wrap_info.iter().enumerate() {
                let max_height = wrap_info.max_height();

                if index == this.selected {
                    this.scroll = this.scroll.clamp(
                        offset_y - ui_state.screen_height + search_bar_height + max_height + 26.0,
                        offset_y
                    );
                    break;
                }

                offset_y += max_height + 26.0;
            }
        };

        let ctrl = ui_state.ctrl;

        let PhysicalKey::Code(keycode) = key.physical_key else { return };

        match (keycode, ctrl) {
            (KeyCode::Escape, _) => self.quit = true,
            (KeyCode::Enter, _) => {
                let _ = self.message_sender.send(Message::Launch(Some(self.entries.list[self.selected].label)));
            }
            (KeyCode::ArrowDown, _) | (KeyCode::KeyJ, true) | (KeyCode::KeyN, true) => {
                self.selected += 1;
                self.selected = self.selected.min(self.entries.list.len().saturating_sub(1));
                snap_selected_to_edge(self);
            }
            (KeyCode::ArrowUp, _) | (KeyCode::KeyK, true) | (KeyCode::KeyP, true) => {
                self.selected = self.selected.saturating_sub(1);
                snap_selected_to_edge(self);
            }
            _ => ()
        }
    }

    pub fn on_cursor_moved(&mut self, window: &Window, pos: PhysicalPosition<f64>) {
        let config = config();
        if let Some(_) = self.hovered_choice {
            window.set_cursor(CursorIcon::Pointer);
        }
        self.input.on_cursor_moved(config, window, pos);
        window.request_redraw();
    }

    pub fn on_left_click(&mut self, window: &Window, ui_state: &crate::UiState) {
        if let Some(hovered_choice) = self.hovered_choice {
            self.message_sender.send(Message::Launch(Some(self.entries.list[hovered_choice].label)))
                .expect("message reciever destroyed");
        } 

        let config = config();
        self.input.on_left_click(config, ui_state);
        window.request_redraw();
    }

    pub fn on_scroll(&mut self, window: &Window, ui_state: &crate::UiState, amount: f64) {
        let config = config();
        let search_bar_height = config.font_size as f64 * 3.25;

        self.scroll -= amount*20.0;
        self.scroll = self.scroll.clamp(0.0, (self.entries.total_height - ui_state.screen_height + search_bar_height).max(0.0));
        window.request_redraw();
    }

    /// Try to call this pretty regularly
    pub fn update(&mut self, rc: &mut RenderContext, window: &Window) {
        let config = config();

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
                Message::IconCacheLoaded(icon_cache) => {
                    self.icons = icon_cache;
                    window.request_redraw();
                }
                Message::Entries(entries) => { 
                    let data = &mut *self.manager.get_data();
                    self.entries = Entries::new(entries, rc, self.theme, &self.font, data);
                    window.request_redraw();
                },
                Message::Action(action) => return self.handle_action(rc, config, action),
            };
        }
    }
}

impl Keal {
    pub fn update_input(&mut self, rc: &mut RenderContext, config: &Config, from_user: bool) {
        self.input.update_input(rc, config, &self.theme, from_user);

        let mut data = self.manager.get_data();
        self.entries.recalculate(rc, self.theme, &self.font, &mut *data);
        drop(data);

        self.manager.send(async_manager::Event::UpdateInput(self.input.text.clone(), from_user));
    }

    fn handle_action(&mut self, rc: &mut RenderContext, config: &Config, action: Action) /* -> Command<Message> */ {
        match action {
            Action::None => (),
            Action::ChangeInput(new) => {
                self.manager.with_manager(|m| m.kill());
                self.input.text = new;
                self.update_input(rc, config, false);
                // return text_input::move_cursor_to_end(text_input::Id::new("query_input"));
            }
            Action::ChangeQuery(new) => {
                let new = self.manager.use_manager(|m| m.current().map(
                    |plugin| format!("{} {}", plugin.prefix, new) 
                )).unwrap_or(new);
                self.input.text = new;
                self.update_input(rc, config, false);
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
