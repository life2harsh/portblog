use std::cell::RefCell;

use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use gloo_events::EventListener;
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{
    AnalyserNode, AudioContext, CanvasRenderingContext2d, HtmlAudioElement, HtmlCanvasElement,
};

const AUDIO_ID: &str = "site-audio";
const TRACK_SRC: &str = "/audio/draft5.mp3";
const TRACK_TITLE: &str = "draft5";
const BARS: usize = 16;
const BAR_WIDTH: f64 = 3.0;
const BAR_GAP: f64 = 2.0;
const BLOCK: f64 = 2.0;
const BLOCK_GAP: f64 = 1.0;
const HEIGHT: f64 = 17.0;

thread_local! {
    static GRAPH: RefCell<Option<(AudioContext, AnalyserNode)>> = const { RefCell::new(None) };
}

pub(crate) fn toggle() {
    let Some(audio) = audio() else {
        return;
    };
    if audio.paused() {
        ensure_graph(&audio);
        GRAPH.with(|graph| {
            if let Some((context, _)) = graph.borrow().as_ref() {
                let _ = context.resume();
            }
        });
        let _ = audio.play();
    } else {
        let _ = audio.pause();
    }
}

fn audio() -> Option<HtmlAudioElement> {
    gloo_utils::document()
        .get_element_by_id(AUDIO_ID)
        .and_then(|element| element.dyn_into::<HtmlAudioElement>().ok())
}

fn ensure_graph(audio: &HtmlAudioElement) {
    GRAPH.with(|graph| {
        if graph.borrow().is_some() {
            return;
        }
        let Ok(context) = AudioContext::new() else {
            return;
        };
        let (Ok(source), Ok(analyser)) = (
            context.create_media_element_source(audio),
            context.create_analyser(),
        ) else {
            return;
        };
        analyser.set_fft_size(256);
        analyser.set_smoothing_time_constant(0.72);
        analyser.set_min_decibels(-115.0);
        analyser.set_max_decibels(-25.0);
        let _ = source.connect_with_audio_node(&analyser);
        let _ = analyser.connect_with_audio_node(&context.destination());
        *graph.borrow_mut() = Some((context, analyser));
    });
}

#[component]
pub(crate) fn Equalizer() -> Element {
    let mut playing = use_signal(|| false);
    let mut canvas = use_signal(|| None::<HtmlCanvasElement>);

    use_effect(move || {
        let Some(audio) = audio() else {
            return;
        };
        let on_play = EventListener::new(&audio, "play", move |_| *playing.write() = true);
        let on_pause = EventListener::new(&audio, "pause", move |_| *playing.write() = false);
        std::mem::forget(on_play);
        std::mem::forget(on_pause);
    });

    use_effect(move || {
        let is_playing = playing();
        let Some(canvas) = canvas() else {
            return;
        };
        if is_playing {
            let generation = bump_generation(&canvas);
            frame(canvas, generation, [(1.0, 0.0); BARS]);
        } else {
            bump_generation(&canvas);
            draw(&canvas, &[0.0; BARS]);
        }
    });

    rsx! {
        div { class: if playing() { "eq is-playing" } else { "eq" },
            button {
                class: "eq__toggle",
                aria_label: if playing() { "Pause music" } else { "Play music" },
                onclick: move |_| toggle(),
                span { class: "eq__glyph", if playing() { "❚❚" } else { "▶" } }
                span { class: "eq__title", "{TRACK_TITLE}" }
                canvas {
                    class: "eq__meter",
                    aria_hidden: "true",
                    onmounted: move |event| {
                        let element: HtmlCanvasElement = event
                            .as_web_event()
                            .dyn_into()
                            .expect("eq canvas mount target");
                        draw(&element, &[0.0; BARS]);
                        *canvas.write() = Some(element);
                    },
                }
            }
            audio { id: AUDIO_ID, src: TRACK_SRC, preload: "none", r#loop: true }
        }
    }
}

fn bump_generation(canvas: &HtmlCanvasElement) -> u32 {
    let next = canvas
        .get_attribute("data-generation")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0)
        .wrapping_add(1);
    let _ = canvas.set_attribute("data-generation", &next.to_string());
    next
}

fn frame(canvas: HtmlCanvasElement, generation: u32, mut bands: [(f64, f64); BARS]) {
    let callback = Closure::once(move || {
        if canvas.get_attribute("data-generation") != Some(generation.to_string()) {
            return;
        }
        if crate::motion::prefers_reduced_motion() {
            draw(&canvas, &[0.35; BARS]);
            return;
        }
        let mut levels = [0.0; BARS];
        GRAPH.with(|graph| {
            let Some((_, analyser)) = graph.borrow().as_ref().cloned() else {
                return;
            };
            let mut bins = vec![0u8; analyser.frequency_bin_count() as usize];
            analyser.get_byte_frequency_data(&mut bins);
            for (bar, (floor, peak)) in bands.iter_mut().enumerate() {
                let start = band_edge(bar, bins.len());
                let end = band_edge(bar + 1, bins.len()).max(start + 1);
                let raw = bins[start..end].iter().copied().max().unwrap_or(0) as f64 / 255.0;
                *peak = raw.max(*peak - 0.0015);
                *floor = raw.min(*floor + 0.0025);
                let range = (*peak - *floor).max(0.12);
                levels[bar] = ((raw - *floor) / range).clamp(0.0, 1.0).powf(1.3);
            }
        });
        draw(&canvas, &levels);
        frame(canvas, generation, bands);
    });
    let _ = gloo_utils::window().request_animation_frame(callback.as_ref().unchecked_ref());
    callback.forget();
}

fn band_edge(bar: usize, bins: usize) -> usize {
    let t = bar as f64 / BARS as f64;
    (14f64.powf(t) as usize + bar).min(bins - 1)
}

fn draw(canvas: &HtmlCanvasElement, levels: &[f64; BARS]) {
    let ratio = gloo_utils::window().device_pixel_ratio().max(1.0);
    let width = BARS as f64 * (BAR_WIDTH + BAR_GAP) - BAR_GAP;
    let (pixel_width, pixel_height) = ((width * ratio) as u32, (HEIGHT * ratio) as u32);
    if canvas.width() != pixel_width || canvas.height() != pixel_height {
        canvas.set_width(pixel_width);
        canvas.set_height(pixel_height);
    }
    let Some(ctx) = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|ctx| ctx.dyn_into::<CanvasRenderingContext2d>().ok())
    else {
        return;
    };
    let _ = ctx.set_transform(ratio, 0.0, 0.0, ratio, 0.0, 0.0);
    ctx.clear_rect(0.0, 0.0, width, HEIGHT);
    let style = gloo_utils::window()
        .get_computed_style(canvas)
        .ok()
        .flatten();
    let color = |name: &str, fallback: &str| {
        style
            .as_ref()
            .and_then(|style| style.get_property_value(name).ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| fallback.to_string())
    };
    let (ink, accent) = (color("--ink", "#141413"), color("--accent-deep", "#6f8f00"));
    let slots = ((HEIGHT + BLOCK_GAP) / (BLOCK + BLOCK_GAP)).floor() as usize;
    for (bar, level) in levels.iter().enumerate() {
        let x = bar as f64 * (BAR_WIDTH + BAR_GAP);
        let lit = ((level * slots as f64).round() as usize).clamp(1, slots);
        for block in 0..lit {
            let y = HEIGHT - BLOCK - block as f64 * (BLOCK + BLOCK_GAP);
            let hot = block + 2 >= slots && lit > 1;
            ctx.set_fill_style_str(if hot { &accent } else { &ink });
            ctx.fill_rect(x, y, BAR_WIDTH, BLOCK);
        }
    }
}
