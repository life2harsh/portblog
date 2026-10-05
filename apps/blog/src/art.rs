use art::ArtEngine;
use content::{ArtConfig, Design};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

pub fn config_for(seed: u64) -> ArtConfig {
    ArtConfig {
        design: Design::ALL[(seed % Design::ALL.len() as u64) as usize],
        seed: seed % 99_991,
    }
}

#[derive(Clone, PartialEq, Props)]
pub struct EntryArtProps {
    pub config: ArtConfig,
    pub accent: &'static str,
    pub ink: &'static str,
}

#[component]
pub fn EntryArt(props: EntryArtProps) -> Element {
    let mut engine: Signal<Option<ArtEngine>> = use_signal(|| None);

    use_effect(use_reactive!(|props| {
        if let Ok(mut slot) = engine.try_write()
            && let Some(engine) = slot.as_mut()
        {
            engine.transition_to(props.config);
            engine.set_accent(props.accent);
            engine.set_ink(props.ink);
        }
    }));

    use_effect(move || {
        fn frame(mut engine: Signal<Option<ArtEngine>>) {
            let callback = wasm_bindgen::closure::Closure::once(move || {
                let Ok(mut slot) = engine.try_write() else {
                    return;
                };
                if let Some(engine) = slot.as_mut() {
                    engine.tick(js_sys::Date::now());
                }
                drop(slot);
                frame(engine);
            });
            let _ = gloo_utils::window().request_animation_frame(callback.as_ref().unchecked_ref());
            callback.forget();
        }
        frame(engine);
    });

    rsx! {
        figure { class: "entry-art", aria_hidden: "true",
            canvas {
                width: 720,
                height: 540,
                onmounted: move |event| {
                    let canvas: HtmlCanvasElement = event
                        .as_web_event()
                        .dyn_into()
                        .expect("art canvas mount target");
                    let mut created = ArtEngine::new(&canvas, props.config, props.accent);
                    created.set_ink(props.ink);
                    created.set_still(reduced_motion());
                    *engine.write() = Some(created);
                },
            }
        }
    }
}

fn reduced_motion() -> bool {
    gloo_utils::window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .is_some_and(|query| query.matches())
}
