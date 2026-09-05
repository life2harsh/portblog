use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use gloo_timers::callback::Timeout;
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};

const BAYER: [f64; 16] = [
    0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0,
];
const RESOLVE_CELLS: [(u32, u32); 3] = [(12, 1), (6, 80), (3, 160)];
const RESOLVED_AT_MS: u32 = 250;

#[component]
pub(crate) fn DitherPreview(
    src: &'static str,
    alt: String,
    #[props(default)] full: bool,
) -> Element {
    rsx! {
        div { class: if full { "dither dither--full" } else { "dither" },
            img { class: "dither__original", src: "{src}", alt: "{alt}", loading: "lazy" }
            canvas {
                class: "dither__canvas",
                aria_hidden: "true",
                onmounted: move |event| {
                    let canvas: HtmlCanvasElement = event
                        .as_web_event()
                        .dyn_into()
                        .expect("dither canvas mount target");
                    load(canvas, src, full);
                },
            }
        }
    }
}

fn load(canvas: HtmlCanvasElement, src: &str, full: bool) {
    let Ok(image) = HtmlImageElement::new() else {
        return;
    };
    let loaded = image.clone();
    let onload = Closure::once(move || {
        let reduced = crate::motion::prefers_reduced_motion();
        if full
            && let Some(frame) = canvas
                .parent_element()
                .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let ratio = loaded.natural_width() as f64 / loaded.natural_height().max(1) as f64;
            let _ = frame
                .style()
                .set_property("--ratio", &format!("{ratio:.4}"));
        }
        render(&canvas, &loaded, RESOLVE_CELLS[2].0);
        if !reduced {
            for &(cell, delay) in &RESOLVE_CELLS {
                let canvas = canvas.clone();
                let image = loaded.clone();
                Timeout::new(delay, move || render(&canvas, &image, cell)).forget();
            }
        }
        Timeout::new(if reduced { 0 } else { RESOLVED_AT_MS }, move || {
            if let Some(frame) = canvas.parent_element() {
                let _ = frame.set_attribute("data-resolved", "true");
            }
        })
        .forget();
    });
    image.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();
    image.set_src(src);
}

fn render(canvas: &HtmlCanvasElement, image: &HtmlImageElement, cell: u32) {
    let width = canvas.client_width().max(1) as u32;
    let height = canvas.client_height().max(1) as u32;
    canvas.set_width(width);
    canvas.set_height(height);
    let columns = width.div_ceil(cell);
    let rows = height.div_ceil(cell);

    let Some(sample) = gloo_utils::document()
        .create_element("canvas")
        .ok()
        .and_then(|element| element.dyn_into::<HtmlCanvasElement>().ok())
    else {
        return;
    };
    sample.set_width(columns);
    sample.set_height(rows);
    let (Some(sample_ctx), Some(ctx)) = (context(&sample), context(canvas)) else {
        return;
    };
    let (source_w, source_h) = (image.natural_width() as f64, image.natural_height() as f64);
    let scale = (columns as f64 / source_w).max(rows as f64 / source_h);
    let (crop_w, crop_h) = (columns as f64 / scale, rows as f64 / scale);
    let _ = sample_ctx
        .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            image,
            (source_w - crop_w) / 2.0,
            0.0,
            crop_w,
            crop_h,
            0.0,
            0.0,
            columns as f64,
            rows as f64,
        );
    let Ok(data) = sample_ctx.get_image_data(0.0, 0.0, columns as f64, rows as f64) else {
        return;
    };
    let pixels = data.data();
    let mut luma = Vec::with_capacity(pixels.len() / 4);
    let mut chroma = Vec::with_capacity(pixels.len() / 4);
    for p in pixels.as_chunks::<4>().0 {
        let (r, g, b) = (p[0] as f64, p[1] as f64, p[2] as f64);
        luma.push((0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0);
        chroma.push((r.max(g).max(b) - r.min(g).min(b)) / 255.0);
    }

    let dark_page = gloo_utils::document()
        .document_element()
        .and_then(|root| root.get_attribute("data-mode"))
        .is_some_and(|mode| mode == "dark");
    if dark_page {
        luma.iter_mut().for_each(|value| *value = 1.0 - *value);
    }
    let mean = luma.iter().sum::<f64>() / luma.len().max(1) as f64;
    let dark = mean < 0.45;
    if let Some(frame) = canvas.parent_element() {
        let _ = frame.set_attribute("data-dark", if dark { "true" } else { "false" });
    }
    if dark {
        luma.iter_mut().for_each(|value| *value = 1.0 - *value);
    }
    let (low, high) = luma.iter().fold((1.0_f64, 0.0_f64), |(low, high), &v| {
        (low.min(v), high.max(v))
    });
    let range = (high - low).max(0.05);

    let ink = css_var(canvas, "--ink").unwrap_or_else(|| "#141413".to_string());
    let accent = css_var(canvas, "--accent-deep").unwrap_or_else(|| ink.clone());
    ctx.clear_rect(0.0, 0.0, width as f64, height as f64);
    for row in 0..rows {
        for column in 0..columns {
            let normalized = (luma[(row * columns + column) as usize] - low) / range;
            let value = ((normalized - 0.5) * 1.3 + 0.54).clamp(0.0, 1.0);
            let threshold = (BAYER[((row % 4) * 4 + column % 4) as usize] + 0.5) / 16.0;
            if value < threshold {
                let colored = chroma[(row * columns + column) as usize] > 0.3;
                ctx.set_fill_style_str(if colored { accent.trim() } else { ink.trim() });
                ctx.fill_rect(
                    (column * cell) as f64,
                    (row * cell) as f64,
                    cell as f64,
                    cell as f64,
                );
            }
        }
    }
}

fn context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|ctx| ctx.dyn_into::<CanvasRenderingContext2d>().ok())
}

fn css_var(element: &HtmlCanvasElement, name: &str) -> Option<String> {
    gloo_utils::window()
        .get_computed_style(element)
        .ok()
        .flatten()
        .and_then(|style| style.get_property_value(name).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
