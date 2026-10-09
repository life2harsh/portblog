use art::ArtEngine;
use content::ArtConfig;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

#[derive(Clone, PartialEq, Props)]
pub struct ArtCanvasProps {
    pub config: ArtConfig,
    pub figure: String,
    pub accent: &'static str,
    pub ink: &'static str,
}

#[component]
pub fn ArtCanvas(props: ArtCanvasProps) -> Element {
    let engine: Signal<Option<ArtEngine>> = use_signal(|| None);
    let mut engine = engine;

    use_effect(use_reactive!(|props| {
        if let Some(engine) = engine.write().as_mut() {
            engine.transition_to(props.config);
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
            div { class: "art__window",
                div { class: "art__bar",
                    svg { class: "art__icon", view_box: "0 0 10 10", "aria-hidden": "true",
                        path { d: "M0.5 0.5h9v9h-9z M2.5 3l2 2-2 2 M5.5 7h2" }
                    }
                    span { class: "art__figure", "[ FIG. {props.figure} ]" }
                    svg { class: "art__icon", view_box: "0 0 10 10", "aria-hidden": "true",
                        path { d: "M1 2.5h8 M1 5h8 M1 7.5h8 M3 1.5v2 M7 4v2 M4.5 6.5v2" }
                    }
                }
                div { class: "art__body",
                    canvas {
                        class: "art__canvas",
                        onmounted: move |event| {
                            let canvas: HtmlCanvasElement = event
                                .as_web_event()
                                .dyn_into()
                                .expect("canvas mount target");
                            let mut created = ArtEngine::new(&canvas, props.config, props.accent);
                            created.set_still(reduced_motion());
                            created.set_ink(props.ink);
                            created.set_solid(true);
                            *engine.write() = Some(created);
                        },
                    }
                }
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
