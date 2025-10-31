mod winit_app;

use config::Theme;
use softbuffer::Surface;

use std::{num::NonZeroU32, rc::Rc, time::Duration};
use winit::{dpi::{LogicalSize, PhysicalPosition}, event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent}, event_loop::{ControlFlow, EventLoop}, raw_window_handle::{HasDisplayHandle, HasWindowHandle}, window::Window};

use piet_tiny_skia::{self as pts, piet::FontFamily, tiny_skia::Pixmap, AsPixmapMut};
use pts::piet::{RenderContext, Text};

mod ui;
mod config;

struct State {
    cache: pts::Cache,
    pixmap: Pixmap,
    ui_state: UiState,
    theme: &'static Theme,
    keal: ui::Keal
}

struct UiState {
    screen_width: f64,
    screen_height: f64,
    mouse_pos: PhysicalPosition<f64>,
    ctrl: bool,
    shift: bool
}

fn redraw<D, W>(state: &mut State, window: &mut Rc<Window>, surface: &mut Surface<D, W>) 
    where D: HasDisplayHandle, W: HasWindowHandle
{
    let size = window.inner_size();
    if size.width == 0 || size.height == 0 { return }
    if state.pixmap.width() != size.width || state.pixmap.height() != size.height {
        state.pixmap = Pixmap::new(size.width, size.height).unwrap();
    }

    let mut render_context = state.cache.render_context(state.pixmap.as_mut());
    render_context.clear(None, state.theme.background);

    state.keal.render(&state.ui_state, &mut render_context);

    let mut buffer = surface.buffer_mut().unwrap();
    for (i, pixel) in state.pixmap.pixels().into_iter().enumerate() {
        buffer[i] = ((pixel.red() as u32) << 16) | ((pixel.green() as u32) << 8) | ((pixel.blue() as u32));
    }

    buffer.present().unwrap();
}

fn main() {
    keal::start_log_time();
    match keal::arguments::Arguments::init("piet") {
        Ok(_) => (),
        Err(keal::arguments::Error::Exit) => return,
        Err(keal::arguments::Error::UnknownFlag(flag)) => {
            panic!("error: unknown flag `{flag}`")
        }
    };

    keal::log_time("reading config");

    let mut theme = config::Theme::default();
    let config = keal::config::Config::init(&mut theme);

    let theme = Box::leak(Box::new(theme));

    keal::log_time("initializing winit");

    let event_loop = EventLoop::new().unwrap();

    let app = winit_app::WinitAppBuilder::with_init(
        |elwt| {
            keal::log_time("initializing window state");

            let window = winit_app::make_window(elwt, |w| w);
            window.set_title("Keal");
            window.set_decorations(false);
            let _ = window.request_inner_size(LogicalSize::new(1920/3, 1080/2));

            let context = softbuffer::Context::new(window.clone()).unwrap();

            let mut cache = pts::Cache::new();
            let mut pixmap = Pixmap::new(1, 1).unwrap();

            keal::log_time("loading font");
            let mut rc = cache.render_context(pixmap.as_pixmap_mut());
            let text = rc.text();
            let font = text.font_family(&config.font).unwrap_or_else(|| {
                eprintln!("couldn't find find the font `{}`, falling back on default font", config.font);
                FontFamily::SYSTEM_UI
            });

            keal::log_time("initializing keal state");
            let keal = ui::Keal::new(&mut rc, font, theme);

            let state = State {
                cache,
                pixmap,
                keal,
                theme,
                ui_state: UiState { 
                    screen_width: 1.0, screen_height: 1.0,
                    mouse_pos: PhysicalPosition::new(0.0, 0.0), ctrl: false, shift: false
                }
            };

            // window.set_ime_allowed(true);

            (window, context, state)
        },
        |_elwt, (window, context, _state)| softbuffer::Surface::new(context, window.clone()).unwrap(),
    );

    let app = app.with_event_handler(|(window, _context, state), surface, event, elwt| {
        elwt.set_control_flow(ControlFlow::wait_duration(Duration::from_millis(30)));

        if state.keal.quit {
            elwt.exit();
            return;
        }

        match event {
            Event::AboutToWait => {
                let mut rc = state.cache.render_context(state.pixmap.as_pixmap_mut());
                state.keal.update(&mut rc, window);
            }
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::RedrawRequested => {
                    let Some(surface) = surface else {
                        eprintln!("RedrawRequested fired before Resumed or after Suspended");
                        return;
                    };

                    redraw(state, window, surface);
                }
                WindowEvent::Resized(size) => {
                    let Some(surface) = surface else {
                        eprintln!("Resized fired before Resumed or after Suspended");
                        return;
                    };

                    if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                    {
                        surface.resize(width, height).unwrap();
                        state.pixmap = Pixmap::new(width.get(), height.get()).unwrap();
                        state.ui_state.screen_width = width.get() as f64;
                        state.ui_state.screen_height = height.get() as f64;

                        let mut rc = state.cache.render_context(state.pixmap.as_pixmap_mut());
                        state.keal.on_resize(&mut rc);
                    }
                }
                WindowEvent::CursorMoved { device_id: _, position: pos }=> {
                    state.ui_state.mouse_pos = pos;
                    state.keal.on_cursor_moved(window, pos);
                }
                WindowEvent::MouseInput { device_id: _, state: ElementState::Pressed, button: MouseButton::Left } => {
                    state.keal.on_left_click(window, &state.ui_state);
                }
                WindowEvent::MouseWheel { device_id: _, delta: MouseScrollDelta::LineDelta(_, delta), phase: winit::event::TouchPhase::Moved } => {
                    state.keal.on_scroll(window, &state.ui_state, delta as f64);
                }
                WindowEvent::KeyboardInput { device_id: _, event: key, is_synthetic: _ } => {
                    if let ElementState::Pressed = key.state {
                        let mut rc = state.cache.render_context(state.pixmap.as_pixmap_mut());
                        state.keal.on_key_press(&mut rc, window, &state.ui_state, key);
                    }
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    state.ui_state.ctrl = modifiers.state().control_key();
                    state.ui_state.shift = modifiers.state().shift_key();
                }
                WindowEvent::CloseRequested => { elwt.exit(); }
                _ => ()
            }
            _ => {}
        }
    });

    winit_app::run_app(event_loop, app);
}
