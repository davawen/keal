#![allow(non_snake_case)]

use std::{fs, collections::HashMap, cell::Cell, rc::Rc};

use dioxus::{prelude::*, html::input_data::keyboard_types::{Code, Modifiers}};
use dioxus_desktop::{Config, WindowBuilder, PhysicalSize};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

fn set_window_properties(cx: Scope) {
    let window = dioxus_desktop::use_window(cx);
    let monitor = window.current_monitor().unwrap();
    let size = monitor.size();
    window.set_inner_size(PhysicalSize { width: size.width / 3, height: size.height / 2 });

    // window.devtool();
}

#[inline_props]
fn List(cx: Scope<ListProps>, filter: String, #[props(!optional)] keyboard: Option<KeyboardData>) -> Element {
    let matcher = use_ref(cx, SkimMatcherV2::default);
    let values = use_ref(cx, || {
        let Ok(path) = std::env::var("PATH") else { return vec![] };

        let mut values = vec![];
        for path in path.split(':') {
            let entries = fs::read_dir(path)
                .map(|entries| entries
                    .flatten()
                    .filter(|entry| !entry.metadata().map(|x| x.is_dir()).unwrap_or(false))
                    .flat_map(|entry| Some(entry.file_name().to_str()?.to_owned()))
                );

            if let Ok(entries) = entries {
                values.extend(entries);
            }
        }

        values
    });

    let selected = use_state(cx, || 0_usize);

    let filtered = use_memo(cx, (matcher, values, filter), |(matcher, values, filter)| {
        let mut scored = values.read().iter()
            .enumerate()
            .flat_map(|(i, filename)| Some((i, matcher.read().fuzzy_match(filename, &filter)?)))
            .collect::<Vec<_>>();

        scored.sort_unstable_by_key(|(_, score)| std::cmp::Reverse(*score));
        scored.truncate(50);

        // limit selected to the number of available choices when refiltering
        selected.set((*selected.get()).min(scored.len().saturating_sub(1)));

        scored
    });

    let highlighted_text = |i: usize| {
        let filename = &values.read()[i];
        let Some((_, indices)) = matcher.read().fuzzy_indices(filename, filter) else { return None };

        let mut idx = 0;

        cx.render( rsx! {
            for (i, c) in filename.chars().enumerate() {
                span {
                    class: if Some(i) == indices.get(idx).copied() {
                        idx += 1;
                        "text-matched"
                    } else {
                        "text-normal"
                    },
                    "{c}"
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
                highlighted_text(element),
            }
        }
    })
}

fn App(cx: Scope) -> Element {
    let filter = use_state(cx, String::new);

    let keyboard = use_state(cx, || None);
    let transmit_keyboard = move |event: Event<KeyboardData>| {
        if matches!(
            (event.code(), event.modifiers()),
            (Code::KeyK | Code::KeyJ, Modifiers::CONTROL)
        ) {
            keyboard.set(Some((*event.data).clone()));
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

fn main() {
    dioxus_desktop::launch_with_props(App, (), Config::new()
        .with_window(WindowBuilder::new()
            .with_resizable(false)
            .with_always_on_top(true)
            .with_transparent(true)
            .with_decorations(false)
            .with_title("Keal")
        )
        .with_custom_head(r#"<link rel="stylesheet" href="public/style.css" />"#.to_owned())
    );
}
