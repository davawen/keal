use dioxus::{prelude::*, html::input_data::keyboard_types::{Code, Modifiers, Key}};
use dioxus_desktop::PhysicalSize;

use crate::search::{self, EntryTrait, Entry};

fn set_window_properties(cx: Scope) {
    let window = dioxus_desktop::use_window(cx);
    let monitor = window.current_monitor().unwrap();
    let size = monitor.size();
    window.set_inner_size(PhysicalSize { width: size.width / 3, height: size.height / 2 });

    // window.devtool();
}

#[inline_props]
fn List(cx: Scope<ListProps>, filter: String, #[props(!optional)] keyboard: Option<KeyboardData>) -> Element {
    let matcher = use_ref(cx, fuzzy_matcher::skim::SkimMatcherV2::default);
    let selected = use_state(cx, || 0_usize);

    let plugins = use_ref(cx, search::plugin::get_plugins);
    let entries = use_ref(cx, || search::create_entries(&plugins.read()));

    let field_entries = use_ref(cx, || None);

    let filtered = use_memo(cx, (filter,), |(filter,)| {
        let filtered = if let Some((plugin, filter)) = plugins.read().filter_starts_with_plugin(&filter) {
            let fields: Vec<_> = plugin.generate().map(Entry::from).collect();
            let filtered = search::filter_entries(&*matcher.read(), &fields, filter, 50);

            field_entries.set(Some(fields));
            filtered
        } else {
            field_entries.set(None);
            search::filter_entries(&*matcher.read(), &entries.read(), &filter, 50)
        };
        
        // limit selected to the number of available choices when refiltering
        selected.set((*selected.get()).min(filtered.len().saturating_sub(1)));

        filtered
    });

    let highlighted_text = |entries: &[Entry], entry: usize| {
        let entry = &entries[entry];
        cx.render( rsx! {
            for (span, highlighted) in entry.fuzzy_match_span(&*matcher.read(), filter) {
                span {
                    class: if highlighted { "text-matched" } else { "text-normal" },
                    "{span}"
                }
            }
        })
    };

    use_memo(cx, (keyboard,), |(keyboard,)| {
        let Some(keyboard) = keyboard else { return };

        match (keyboard.code(), keyboard.modifiers()) {
            (Code::KeyJ, Modifiers::CONTROL) => selected.set((selected.get() + 1).min(filtered.len().saturating_sub(1))),
            (Code::KeyK, Modifiers::CONTROL) => selected.set(selected.get().saturating_sub(1)),
            _ => unreachable!()
        }
    });

    cx.render(rsx! {
        for (i, &(element, _)) in filtered.iter().enumerate() {
            div {
                key: "{i}",
                class: if *selected.get() == i {
                    "no-select item selected"
                } else { "no-select item" },
                div {
                    class: "name",
                    highlighted_text(field_entries.read().as_ref().unwrap_or(&entries.read()), element)
                }
                div {
                    class: "comment",
                    if let Some(comment) = entries.read()[element].comment() {
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
            (_, Code::KeyK | Code::KeyJ, Modifiers::CONTROL) => keyboard.set(Some((*event.data).clone())),
            (Key::Escape, ..) => dioxus_desktop::use_window(cx).close(),
            (Key::Enter, ..) => todo!("launch application"),
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
                    filter: filter.get().clone(),
                    keyboard: if keyboard.get().is_some() {
                        keyboard.make_mut().take()
                    } else { None }
                }
            }
        }
    })
}

