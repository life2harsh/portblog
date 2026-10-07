use std::rc::Rc;

use content::Project;
use dioxus::prelude::*;
use gloo_events::{EventListener, EventListenerOptions};
use wasm_bindgen::JsCast;

use crate::preview::DitherPreview;

pub(crate) fn viewer_open() -> bool {
    gloo_utils::document()
        .query_selector(".viewer")
        .ok()
        .flatten()
        .is_some()
}

#[component]
pub(crate) fn Gallery(project: &'static Project) -> Element {
    let mut index = use_signal(|| 0usize);
    let mut open = use_signal(|| false);
    let origin = use_signal(|| None);
    let count = project.previews.len();
    let current = project.previews[index() % count];
    let label = if project.live_url.is_some() {
        "LIVE SITE".to_string()
    } else {
        project.name.to_uppercase()
    };
    rsx! {
        figure { class: "detail__preview",
            button {
                class: "gallery__thumb",
                aria_label: "Expand {project.name} screenshots",
                onclick: move |_| *open.write() = true,
                ontouchstart: move |event| crate::swipe::begin(origin, &event),
                ontouchend: move |event| {
                    if let Some(delta) = crate::swipe::end(origin, &event).and_then(|gesture| gesture.step()) {
                        event.stop_propagation();
                        *index.write() = (index() as i64 + delta).rem_euclid(count as i64) as usize;
                    }
                },
                for src in std::iter::once(current) {
                    DitherPreview { key: "{src}", src, alt: format!("{} screenshot", project.name) }
                }
            }
            figcaption { class: "gallery__caption",
                span { "PREVIEW  {label}" }
                if count > 1 {
                    Stepper { index, count }
                }
            }
        }
        if open() {
            Viewer { project, index, on_close: move |_| *open.write() = false }
        }
    }
}

#[component]
fn Stepper(index: Signal<usize>, count: usize) -> Element {
    let mut index = index;
    rsx! {
        span { class: "gallery__stepper",
            button { aria_label: "Previous screenshot", onclick: move |_| *index.write() = (index() + count - 1) % count, "←" }
            span { class: "gallery__dots", aria_label: "Screenshot {index() + 1} of {count}",
                for dot in 0..count {
                    i { class: if dot == index() % count { "is-current" } else { "" } }
                }
            }
            button { aria_label: "Next screenshot", onclick: move |_| *index.write() = (index() + 1) % count, "→" }
        }
    }
}

#[component]
fn Viewer(project: &'static Project, index: Signal<usize>, on_close: EventHandler<()>) -> Element {
    let mut index = index;
    let count = project.previews.len();
    let current = project.previews[index() % count];
    let origin = use_signal(|| None);
    let close = move || {
        if crate::nav::layer() >= 2 {
            crate::nav::back_to(1);
        } else {
            on_close.call(());
        }
    };

    use_hook(move || {
        crate::nav::push(2);
        Rc::new((
            EventListener::new(&gloo_utils::window(), "popstate", move |_| {
                if crate::nav::layer() < 2 {
                    on_close.call(());
                }
            }),
            EventListener::new_with_options(
                &gloo_utils::document(),
                "keydown",
                EventListenerOptions::enable_prevent_default(),
                move |event| {
                    let Some(key) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                        return;
                    };
                    match key.key().as_str() {
                        "Escape" | "q" => close(),
                        "ArrowLeft" | "h" | "k" => *index.write() = (index() + count - 1) % count,
                        "ArrowRight" | "l" | "j" => *index.write() = (index() + 1) % count,
                        _ => return,
                    }
                    key.prevent_default();
                },
            ),
        ))
    });

    rsx! {
        div {
            class: "viewer",
            role: "dialog",
            aria_modal: "true",
            aria_label: "{project.name} screenshots",
            onclick: move |_| close(),
            header { class: "viewer__bar",
                span { "{project.name.to_lowercase()}  screenshots" }
                button { onclick: move |_| close(), span { class: "key-hint", "[esc] " } "close" }
            }
            div {
                class: "viewer__stage",
                onclick: move |event| event.stop_propagation(),
                ontouchstart: move |event| crate::swipe::begin(origin, &event),
                ontouchend: move |event| match crate::swipe::end(origin, &event) {
                    Some(crate::swipe::Gesture::Down) => close(),
                    Some(gesture) => {
                        if let Some(delta) = gesture.step() {
                            *index.write() = (index() as i64 + delta).rem_euclid(count as i64) as usize;
                        }
                    }
                    None => {}
                },
                for src in std::iter::once(current) {
                    DitherPreview { key: "{src}", src, alt: format!("{} screenshot {}", project.name, index() + 1), full: true }
                }
            }
            footer { class: "viewer__bar", onclick: move |event| event.stop_propagation(),
                if count > 1 {
                    button { onclick: move |_| *index.write() = (index() + count - 1) % count, "← prev" }
                    span { class: "viewer__ticks",
                        for tick in 0..count {
                            button {
                                class: if tick == index() % count { "is-current" } else { "" },
                                aria_label: "Screenshot {tick + 1}",
                                onclick: move |_| *index.write() = tick,
                                "{tick + 1:02}"
                            }
                        }
                    }
                    button { onclick: move |_| *index.write() = (index() + 1) % count, "next →" }
                } else {
                    span {}
                    span {}
                    span {}
                }
            }
        }
    }
}
