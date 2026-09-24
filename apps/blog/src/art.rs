use art::ArtEngine;
use content::{ArtConfig, ShapeFamily};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

const FAMILIES: [ShapeFamily; 4] = [
    ShapeFamily::Knot,
    ShapeFamily::Orbit,
    ShapeFamily::Lattice,
    ShapeFamily::Ribbon,
];

pub fn config_for(seed: u64) -> ArtConfig {
    let unit = |shift: u32| ((seed >> shift) & 0xff) as f64 / 255.0;
    ArtConfig {
        shape: FAMILIES[(seed % 4) as usize],
        rotation_x: -0.6 + unit(8) * 1.3,
        rotation_y: -0.7 + unit(16) * 1.4,
        twist: 1.0 + unit(24) * 3.0,
        radius: 0.9 + unit(32) * 0.25,
        secondary_radius: 0.26 + unit(40) * 0.2,
        density: 60,
        warp: 0.08 + unit(48) * 0.3,
        symmetry: 2.0 + (unit(56) * 4.0).round(),
        phase: unit(4) * 3.0,
        scale: 1.3,
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
            engine.transition_to(props.config.clone());
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
                    let mut created = ArtEngine::new(&canvas, props.config.clone(), props.accent);
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
