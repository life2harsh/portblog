use content::ArtConfig;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::field::Field;
use crate::math::lerp;

const INK: [f64; 3] = [20.0, 20.0, 19.0];

pub struct ArtEngine {
    ctx: CanvasRenderingContext2d,
    field: Field,
    current_tint: [f64; 3],
    target_tint: [f64; 3],
    current_ink: [f64; 3],
    target_ink: [f64; 3],
    last_time: Option<f64>,
    still: bool,
}

impl ArtEngine {
    pub fn new(canvas: &HtmlCanvasElement, config: ArtConfig, accent: &str) -> Self {
        let ctx = canvas
            .get_context("2d")
            .expect("getContext failed")
            .expect("2d context is null")
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("dyn_into failed");

        Self {
            ctx,
            field: Field::new(config),
            current_tint: hex_rgb(accent),
            target_tint: hex_rgb(accent),
            current_ink: INK,
            target_ink: INK,
            last_time: None,
            still: false,
        }
    }

    pub fn transition_to(&mut self, target: ArtConfig) {
        self.field.transition_to(target);
    }

    pub fn set_accent(&mut self, accent: &str) {
        self.target_tint = hex_rgb(accent);
    }

    pub fn set_ink(&mut self, ink: &str) {
        self.target_ink = hex_rgb(ink);
    }

    pub fn set_still(&mut self, still: bool) {
        self.still = still;
    }

    pub fn tick(&mut self, now_ms: f64) {
        let now = now_ms / 1000.0;
        let dt = self.last_time.map_or(0.0, |last| (now - last).min(0.08));
        self.last_time = Some(now);
        let factor = self.field.step(dt, !self.still);
        for (current, target) in self.current_tint.iter_mut().zip(self.target_tint) {
            *current += (target - *current) * factor;
        }
        for (current, target) in self.current_ink.iter_mut().zip(self.target_ink) {
            *current += (target - *current) * factor;
        }
        self.draw();
    }

    fn draw(&self) {
        let canvas = self.ctx.canvas().expect("canvas is null");
        let width = canvas.width() as f64;
        let height = canvas.height() as f64;
        self.ctx.clear_rect(0.0, 0.0, width, height);
        self.ctx.set_line_width(0.55 * (width / 960.0).max(1.0));
        for ring in self.field.rings(width, height) {
            self.ctx.begin_path();
            for (index, (x, y)) in ring.points.iter().enumerate() {
                if index == 0 {
                    self.ctx.move_to(*x, *y);
                } else {
                    self.ctx.line_to(*x, *y);
                }
            }
            let rgb = [
                lerp(self.current_ink[0], self.current_tint[0], ring.mix),
                lerp(self.current_ink[1], self.current_tint[1], ring.mix),
                lerp(self.current_ink[2], self.current_tint[2], ring.mix),
            ];
            let alpha = (0.28 + ring.depth * 0.55 + ring.mix * 0.15).min(0.9);
            self.ctx.set_stroke_style_str(&format!(
                "rgb({:.0} {:.0} {:.0} / {alpha:.3})",
                rgb[0], rgb[1], rgb[2]
            ));
            self.ctx.stroke();
        }
    }
}

fn hex_rgb(hex: &str) -> [f64; 3] {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return INK;
    }
    let channel =
        |range: std::ops::Range<usize>| u8::from_str_radix(&hex[range], 16).map_or(0.0, f64::from);
    [channel(0..2), channel(2..4), channel(4..6)]
}
