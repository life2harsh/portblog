use art::ArtEngine;
use content::ArtConfig;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

#[derive(Clone, PartialEq, Props)]
pub struct ArtCanvasProps {
    pub config: ArtConfig,
    pub accent: &'static str,
    pub ink: &'static str,
}

#[component]
pub fn ArtCanvas(props: ArtCanvasProps) -> Element {
    let engine: Signal<Option<ArtEngine>> = use_signal(|| None);
    let mut engine = engine;
    let seed = props.config.seed;

    use_effect(use_reactive!(|props| {
        if let Some(engine) = engine.write().as_mut() {
            engine.transition_to(props.config.clone());
            engine.set_accent(props.accent);
            engine.set_ink(props.ink);
        }
    }));

    {
        use_effect(move || {
            let performance = gloo_utils::window()
                .performance()
                .expect("performance unavailable");
            fn frame(mut engine: Signal<Option<ArtEngine>>, performance: web_sys::Performance) {
                let next_performance = performance.clone();
                let callback = wasm_bindgen::closure::Closure::once(move || {
                    if let Some(engine) = engine.write().as_mut() {
                        engine.tick(next_performance.now());
                    }
                    frame(engine, next_performance);
                });
                let _ =
                    gloo_utils::window().request_animation_frame(callback.as_ref().unchecked_ref());
                callback.forget();
            }
            frame(engine, performance);
        });
    }

    rsx! {
        figure { class: "art", aria_label: "Generative ring sculpture",
            div { class: "art__label art__label--top", "GENERATIVE FIELD" br {} "SEED {seed:05}" }
            span { class: "art__mark art__mark--a", "+" }
            span { class: "art__mark art__mark--b", "+" }
            span { class: "art__mark art__mark--c", "+" }
            span { class: "art__mark art__mark--d", "+" }
            span { class: "art__tick" }
            canvas {
                class: "art__canvas",
                width: 960,
                height: 720,
                onmounted: move |event| {
                    let canvas: HtmlCanvasElement = event
                        .as_web_event()
                        .dyn_into()
                        .expect("canvas mount target");
                    let mut created = ArtEngine::new(&canvas, props.config.clone(), props.accent);
                    created.set_still(reduced_motion());
                    created.set_ink(props.ink);
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
