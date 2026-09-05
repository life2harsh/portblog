use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{CanvasRenderingContext2d, Element, HtmlCanvasElement, HtmlElement};

const HIGHLIGHT_SPEED: f64 = 8.8;
const SCROLL_SPEED: f64 = 9.4;

pub fn retarget_highlight(plate: &Element, row: &Element) {
    let Some(row) = row.dyn_ref::<HtmlElement>() else {
        return;
    };
    let target_y = row.offset_top() as f64;
    let target_height = row.offset_height() as f64;

    if prefers_reduced_motion() || plate.get_attribute("data-highlight-y").is_none() {
        set_highlight(plate, target_y, target_height);
        return;
    }

    let _ = plate.set_attribute("data-highlight-target-y", &target_y.to_string());
    let _ = plate.set_attribute("data-highlight-target-height", &target_height.to_string());
    if plate.get_attribute("data-highlight-running").is_none() {
        let _ = plate.set_attribute("data-highlight-running", "true");
        highlight_frame(plate.clone(), now());
    }
}

pub fn retarget_scroll(element: &Element, target: f64) {
    if prefers_reduced_motion() {
        element.set_scroll_top(target.round() as i32);
        return;
    }
    let _ = element.set_attribute("data-scroll-target", &target.to_string());
    if element.get_attribute("data-scroll-running").is_none() {
        let _ = element.set_attribute("data-scroll-running", "true");
        scroll_frame(element.clone(), now());
    }
}

fn highlight_frame(plate: Element, previous_time: f64) {
    let callback = Closure::wrap(Box::new(move || {
        let next_plate = plate.clone();
        let time = now();
        let dt = ((time - previous_time) / 1_000.0).clamp(0.0, 0.05);
        let factor = 1.0 - (-HIGHLIGHT_SPEED * dt).exp();
        let current_y = attribute_number(&plate, "data-highlight-y");
        let current_height = attribute_number(&plate, "data-highlight-height");
        let target_y = attribute_number(&plate, "data-highlight-target-y");
        let target_height = attribute_number(&plate, "data-highlight-target-height");
        let next_y = current_y + (target_y - current_y) * factor;
        let next_height = current_height + (target_height - current_height) * factor;
        set_highlight(&plate, next_y, next_height);

        if (target_y - next_y).abs() < 0.3 && (target_height - next_height).abs() < 0.3 {
            set_highlight(&plate, target_y, target_height);
            let _ = plate.remove_attribute("data-highlight-running");
        } else {
            highlight_frame(next_plate, time);
        }
    }) as Box<dyn FnMut()>);
    let _ = window().request_animation_frame(callback.as_ref().unchecked_ref());
    callback.forget();
}

fn scroll_frame(element: Element, previous_time: f64) {
    let callback = Closure::wrap(Box::new(move || {
        let next_element = element.clone();
        let time = now();
        let dt = ((time - previous_time) / 1_000.0).clamp(0.0, 0.05);
        let target = attribute_number(&element, "data-scroll-target");
        let current = element.scroll_top() as f64;
        let next = current + (target - current) * (1.0 - (-SCROLL_SPEED * dt).exp());
        element.set_scroll_top(next.round() as i32);

        if (target - next).abs() < 0.75 {
            element.set_scroll_top(target.round() as i32);
            let _ = element.remove_attribute("data-scroll-running");
        } else {
            scroll_frame(next_element, time);
        }
    }) as Box<dyn FnMut()>);
    let _ = window().request_animation_frame(callback.as_ref().unchecked_ref());
    callback.forget();
}

fn set_highlight(plate: &Element, y: f64, height: f64) {
    let Some(plate) = plate.dyn_ref::<HtmlElement>() else {
        return;
    };
    let _ = plate
        .style()
        .set_property("transform", &format!("translate3d(0, {y:.2}px, 0)"));
    let _ = plate
        .style()
        .set_property("height", &format!("{height:.2}px"));
    let _ = plate.set_attribute("data-highlight-y", &y.to_string());
    let _ = plate.set_attribute("data-highlight-height", &height.to_string());
}

fn attribute_number(element: &Element, name: &str) -> f64 {
    element
        .get_attribute(name)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0)
}

fn now() -> f64 {
    window()
        .performance()
        .expect("performance unavailable")
        .now()
}

fn window() -> web_sys::Window {
    gloo_utils::window()
}

pub(crate) fn prefers_reduced_motion() -> bool {
    window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .is_some_and(|query| query.matches())
}

pub const VEIL_COVER_MS: u32 = 150;
pub const VEIL_REVEAL_MS: u32 = 260;
const VEIL_CELL: f64 = 9.0;
const BAYER: [f64; 16] = [
    0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0,
];

#[component]
pub(crate) fn PixelVeil(motion: crate::DetailMotion) -> dioxus::prelude::Element {
    let mut canvas = use_signal(|| None::<HtmlCanvasElement>);
    use_effect(use_reactive!(|motion| {
        let Some(canvas) = canvas() else {
            return;
        };
        let generation = attribute_number(&canvas, "data-veil-generation") + 1.0;
        let _ = canvas.set_attribute("data-veil-generation", &generation.to_string());
        match motion {
            crate::DetailMotion::Leaving if !prefers_reduced_motion() => {
                veil_frame(canvas, generation, now(), VEIL_COVER_MS as f64, true);
            }
            crate::DetailMotion::Entering if !prefers_reduced_motion() => {
                veil_frame(canvas, generation, now(), VEIL_REVEAL_MS as f64, false);
            }
            _ => clear_veil(&canvas),
        }
    }));
    rsx! {
        canvas {
            class: "pixel-veil",
            aria_hidden: "true",
            onmounted: move |event| {
                let element: HtmlCanvasElement = event
                    .as_web_event()
                    .dyn_into()
                    .expect("veil mount target");
                *canvas.write() = Some(element);
            },
        }
    }
}

fn clear_veil(canvas: &HtmlCanvasElement) {
    if let Some(ctx) = veil_context(canvas) {
        ctx.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    }
}

fn veil_context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|ctx| ctx.dyn_into::<CanvasRenderingContext2d>().ok())
}

fn veil_frame(
    canvas: HtmlCanvasElement,
    generation: f64,
    start: f64,
    duration: f64,
    covering: bool,
) {
    let callback = Closure::once(move || {
        if attribute_number(&canvas, "data-veil-generation") != generation {
            return;
        }
        let width = canvas.client_width().max(1) as u32;
        let height = canvas.client_height().max(1) as u32;
        if canvas.width() != width || canvas.height() != height {
            canvas.set_width(width);
            canvas.set_height(height);
        }
        let Some(ctx) = veil_context(&canvas) else {
            return;
        };
        let progress = ((now() - start) / duration).clamp(0.0, 1.0);
        let fill = window()
            .get_computed_style(&canvas)
            .ok()
            .flatten()
            .and_then(|style| style.get_property_value("--veil").ok())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "#efede6".to_string());

        ctx.clear_rect(0.0, 0.0, width as f64, height as f64);
        ctx.set_fill_style_str(fill.trim());
        let columns = (width as f64 / VEIL_CELL).ceil() as u32;
        let rows = (height as f64 / VEIL_CELL).ceil() as u32;
        for row in 0..rows {
            for column in 0..columns {
                let order = BAYER[((row % 4) * 4 + column % 4) as usize] / 16.0;
                let jitter = cell_hash(column, row) / 16.0;
                let threshold = order + jitter;
                let shown = if covering {
                    threshold < progress
                } else {
                    threshold >= progress
                };
                if shown {
                    ctx.fill_rect(
                        column as f64 * VEIL_CELL,
                        row as f64 * VEIL_CELL,
                        VEIL_CELL,
                        VEIL_CELL,
                    );
                }
            }
        }
        if progress < 1.0 {
            veil_frame(canvas, generation, start, duration, covering);
        } else if !covering {
            ctx.clear_rect(0.0, 0.0, width as f64, height as f64);
        }
    });
    let _ = window().request_animation_frame(callback.as_ref().unchecked_ref());
    callback.forget();
}

fn cell_hash(column: u32, row: u32) -> f64 {
    let mut n = column.wrapping_mul(374_761_393) ^ row.wrapping_mul(668_265_263);
    n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
    (n % 1000) as f64 / 1000.0
}
