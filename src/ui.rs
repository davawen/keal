use std::io::Write;

use dioxus::{prelude::*, html::input_data::keyboard_types::{Code, Modifiers, Key}};
use dioxus_desktop::PhysicalSize;

use fuzzy_matcher::skim::SkimMatcherV2;
use lazy_static::lazy_static;

use crate::search::{self, EntryTrait, Entry};

fn set_window_properties(cx: Scope) {
    let window = dioxus_desktop::use_window(cx);
    let monitor = window.current_monitor().unwrap();
    let size = monitor.size();
    window.set_inner_size(PhysicalSize { width: size.width / 3, height: size.height / 2 });

    // window.devtool();
}

#[derive(Props, PartialEq)]
struct ListProps {
    filter: String,
    #[props(!optional)]
    keyboard: Option<KeyboardData>
}

fn List(cx: Scope<ListProps>) -> Element {
    let selected = use_state(cx, || 0_usize);

    lazy_static! {
        static ref MATCHER: SkimMatcherV2 = SkimMatcherV2::default();
        static ref PLUGINS: search::plugin::Plugins = search::plugin::get_plugins();
        static ref ENTRIES: Vec<search::Entry> = search::create_entries(&PLUGINS);
    }

    // let plugin_execution = use_ref(cx, || None);
    // let filter = use_state(cx, || cx.props.filter.to_owned());
    let filtered = use_state(cx, Vec::new);
    let plugin_execution = use_ref(cx, || None);

    let filter = use_memo(cx, (&cx.props.filter,), |(prop_filter,)| {
        let (plugin, filter) = if let Some((plugin, remaining)) = PLUGINS.filter_starts_with_plugin(&prop_filter) {
            (Some(plugin.generate()), remaining.to_owned())
        } else {
            (None, prop_filter)
        };

        filtered.set(match &plugin {
            Some(execution) => search::filter_entries(&*MATCHER, &execution.entries, &filter, 50),
            None            => search::filter_entries(&*MATCHER, &ENTRIES, &filter, 50),
        });

        // limit selected to the number of available choices when refiltering
        selected.set((*selected.get()).min(filtered.len().saturating_sub(1)));

        plugin_execution.set(plugin);
        filter
    });


    let window = dioxus_desktop::use_window(cx);
    let interact = || {
        let selected = *selected.get();
        let selected = filtered[selected].0;
        let mut plugin_execution = plugin_execution.write_silent();

        match plugin_execution.as_mut() {
            Some(plugin) => match &plugin.entries[selected] {
                Entry::FieldEntry(field) => {
                    let _ = writeln!(plugin.stdin, "{}", field.name());
                    let _ = plugin.child.wait();
                    window.close();
                }
                _ => unreachable!("something went terribly wrong")
            }
            None => match &ENTRIES[selected] {
                Entry::PluginEntry(plugin) => {
                    todo!("fill in plugin into input")
                }
                Entry::DesktopEntry(app) => {
                    todo!("launch application")
                }
                _ => unreachable!("something went terribly wrong")
            }
        }
    };

    use_memo(cx, (&cx.props.keyboard,), |(keyboard,)| {
        let Some(keyboard) = keyboard else { return };

        match (keyboard.code(), keyboard.modifiers()) {
            (Code::KeyJ, Modifiers::CONTROL) => selected.set((selected.get() + 1).min(filtered.len().saturating_sub(1))),
            (Code::KeyK, Modifiers::CONTROL) => selected.set(selected.get().saturating_sub(1)),
            (Code::Enter, _)                 => interact(),
            _ => unreachable!()
        }
    });

    let highlighted_text = |entry: &Entry| {
        cx.render( rsx! {
            for (span, highlighted) in entry.fuzzy_match_span(&*MATCHER, filter) {
                span {
                    class: if highlighted { "text-matched" } else { "text-normal" },
                    "{span}"
                }
            }
        })
    };

    // it's just not possible to get lifetimes to work with this inside the `use_memo` :(
    let plugin_execution = plugin_execution.read();
    let entries = match &*plugin_execution {
        Some(execution) => &execution.entries,
        None => &ENTRIES
    };

    cx.render(rsx! {
        for (i, &(element, _)) in filtered.iter().enumerate() {
            div {
                key: "{i}",
                class: if *selected.get() == i {
                    "no-select item selected"
                } else { "no-select item" },
                div {
                    class: "name",
                    highlighted_text(&entries[element])
                }
                div {
                    class: "comment",
                    if let Some(comment) = entries[element].comment() {
                        comment
                    } else { "" }
                }
            }
        }
    })
}

pub fn App(cx: Scope) -> Element {
    let filter = use_state(cx, String::new);

    let keyboard = use_state(cx, || None);
    let transmit_keyboard = move |event: Event<KeyboardData>| {
        match (event.key(), event.code(), event.modifiers()) {
            (_, Code::KeyK | Code::KeyJ, Modifiers::CONTROL) | (_, Code::Enter, _) => keyboard.set(Some((*event.data).clone())),
            (Key::Escape, ..) => dioxus_desktop::use_window(cx).close(),
            _ => ()
        }
    };

    let window = dioxus_desktop::use_window(cx);
    let size = window.inner_size();

    cx.render(rsx! {
        div {
            onmounted: move |_| set_window_properties(cx),
            onkeydown: transmit_keyboard,
            // onkeyup: move |_| keyboard.set(None),
            class: "root",
            style: "max-height: {size.height - 15}px",
            input {
                oninput: move |event| filter.set(event.value.clone()),
                onmounted: move |event| { event.set_focus(true); },
                placeholder: "search your dreams!",
                value: "{filter}"
            },
            div {
                class: "list",
                List {
                    filter: filter.get().to_owned(),
                    keyboard: if keyboard.get().is_some() {
                        keyboard.make_mut().take()
                    } else { None }
                }
            }
        }
    })
}

