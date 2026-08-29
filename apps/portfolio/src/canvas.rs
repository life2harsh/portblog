use art::ArtEngine;
use content::project::ArtConfig;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

#[derive(Clone, PartialEq, Props)]
pub struct ArtCanvasProps {
    pub config: ArtConfig,
    pub art_mode_open: bool,
    pub on_art_mode_close: EventHandler<()>,
}

#[component]
pub fn ArtCanvas(props: ArtCanvasProps) -> Element {
    let engine: Signal<Option<ArtEngine>> = use_signal(|| None);
    let mut engine = engine;

    let freq_override = use_signal(|| None::<f64>);
    let amp_override = use_signal(|| None::<f64>);
    let count_override = use_signal(|| None::<u32>);
    let twist_override = use_signal(|| None::<f64>);

    let effective_config = {
        let mut c = props.config.clone();
        if let Some(f) = *freq_override.read() {
            c.frequency = f;
        }
        if let Some(a) = *amp_override.read() {
            c.amplitude = a;
        }
        if let Some(n) = *count_override.read() {
            c.count = n;
        }
        if let Some(t) = *twist_override.read() {
            c.twist = t;
        }
        c
    };

    {
        let eff = effective_config.clone();
        use_effect(move || {
            if let Some(ref mut eng) = *engine.write() {
                eng.transition_to(eff.clone(), 1.0);
            }
        });
    }

    {
        use_effect(move || {
            let window = gloo_utils::window();
            let perf = window.performance().expect("performance not available");

            fn schedule_frame(
                mut engine_sig: Signal<Option<ArtEngine>>,
                perf: web_sys::Performance,
            ) {
                let perf_clone = perf.clone();
                let cb = wasm_bindgen::closure::Closure::once(move || {
                    let now = perf_clone.now();
                    if let Some(ref mut eng) = *engine_sig.write() {
                        eng.tick(now);
                    }
                    schedule_frame(engine_sig, perf_clone);
                });

                gloo_utils::window()
                    .request_animation_frame(cb.as_ref().unchecked_ref())
                    .ok();
                cb.forget();
            }

            schedule_frame(engine, perf);
        });
    }

    rsx! {
        div { class: "canvas-wrap",
            canvas {
                class: "art-canvas",
                id: "art-canvas",
                width: 640,
                height: 360,
                aria_hidden: "true",
                onmounted: move |evt| {
                    let el = evt.as_web_event();
                    let canvas: HtmlCanvasElement = el
                        .dyn_into::<HtmlCanvasElement>()
                        .expect("not a canvas element");
                    let eng = ArtEngine::new(&canvas, props.config.clone());
                    *engine.write() = Some(eng);
                },
            }
            span { class: "canvas-label", "art · press a" }
        }

        if props.art_mode_open {
            ArtModePanel {
                config: effective_config.clone(),
                freq_override,
                amp_override,
                count_override,
                twist_override,
                on_close: props.on_art_mode_close,
            }
        }
    }
}

#[derive(Clone, PartialEq, Props)]
struct ArtModePanelProps {
    config: ArtConfig,
    freq_override: Signal<Option<f64>>,
    amp_override: Signal<Option<f64>>,
    count_override: Signal<Option<u32>>,
    twist_override: Signal<Option<f64>>,
    on_close: EventHandler<()>,
}

#[component]
fn ArtModePanel(props: ArtModePanelProps) -> Element {
    let mut freq_override = props.freq_override;
    let mut amp_override = props.amp_override;
    let mut count_override = props.count_override;
    let mut twist_override = props.twist_override;

    let freq = freq_override.read().unwrap_or(props.config.frequency);
    let amp = amp_override.read().unwrap_or(props.config.amplitude);
    let count = count_override.read().unwrap_or(props.config.count);
    let twist = twist_override.read().unwrap_or(props.config.twist);

    rsx! {
        div { class: "art-mode", role: "region", aria_label: "Art mode controls",
            header { class: "art-mode__heading",
                "art mode"
                button {
                    style: "float: right; color: var(--color-ink-3); font-size: 1rem;",
                    aria_label: "Close art mode",
                    onclick: move |_| props.on_close.call(()),
                    "×"
                }
            }

            div { class: "art-param",
                label { class: "art-param__label", r#for: "param-freq", "frequency" }
                input {
                    class: "art-param__range",
                    id: "param-freq",
                    r#type: "range",
                    min: "0.1", max: "8.0", step: "0.05",
                    value: "{freq:.2}",
                    oninput: move |e| {
                        if let Ok(v) = e.value().parse::<f64>() {
                            *freq_override.write() = Some(v);
                        }
                    },
                }
                span { class: "art-param__value", "{freq:.2}" }
            }

            div { class: "art-param",
                label { class: "art-param__label", r#for: "param-amp", "amplitude" }
                input {
                    class: "art-param__range",
                    id: "param-amp",
                    r#type: "range",
                    min: "0.0", max: "1.2", step: "0.01",
                    value: "{amp:.2}",
                    oninput: move |e| {
                        if let Ok(v) = e.value().parse::<f64>() {
                            *amp_override.write() = Some(v);
                        }
                    },
                }
                span { class: "art-param__value", "{amp:.2}" }
            }

            div { class: "art-param",
                label { class: "art-param__label", r#for: "param-count", "lines" }
                input {
                    class: "art-param__range",
                    id: "param-count",
                    r#type: "range",
                    min: "4", max: "120", step: "1",
                    value: "{count}",
                    oninput: move |e| {
                        if let Ok(v) = e.value().parse::<u32>() {
                            *count_override.write() = Some(v);
                        }
                    },
                }
                span { class: "art-param__value", "{count}" }
            }

            div { class: "art-param",
                label { class: "art-param__label", r#for: "param-twist", "twist" }
                input {
                    class: "art-param__range",
                    id: "param-twist",
                    r#type: "range",
                    min: "0.0", max: "4.0", step: "0.05",
                    value: "{twist:.2}",
                    oninput: move |e| {
                        if let Ok(v) = e.value().parse::<f64>() {
                            *twist_override.write() = Some(v);
                        }
                    },
                }
                span { class: "art-param__value", "{twist:.2}" }
            }

            div { class: "art-mode__actions",
                button {
                    class: "art-btn",
                    onclick: move |_| {
                        *freq_override.write() = None;
                        *amp_override.write() = None;
                        *count_override.write() = None;
                        *twist_override.write() = None;
                    },
                    "reset"
                }
            }
        }
    }
}
