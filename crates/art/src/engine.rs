use content::ArtConfig;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::field::Field;
use crate::math::lerp;

const INK: [f64; 3] = [20.0, 20.0, 19.0];
const PAPER: [f64; 3] = [239.0, 237.0, 230.0];

pub struct ArtEngine {
    ctx: CanvasRenderingContext2d,
    field: Field,
    current_tint: [f64; 3],
    target_tint: [f64; 3],
    current_ink: [f64; 3],
    target_ink: [f64; 3],
    paper: [f64; 3],
    frames: u64,
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
            paper: PAPER,
            frames: 0,
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
        if self.frames.is_multiple_of(20) {
            let canvas = self.ctx.canvas().expect("canvas is null");
            self.paper = read_paper(&canvas).unwrap_or(self.paper);
        }
        self.frames += 1;
        self.draw();
    }

    fn fit(&self, canvas: &HtmlCanvasElement) -> f64 {
        let ratio = web_sys::window()
            .map_or(1.0, |window| window.device_pixel_ratio())
            .clamp(1.0, 2.0);
        let width = (canvas.client_width() as f64 * ratio).round() as u32;
        let height = (canvas.client_height() as f64 * ratio).round() as u32;
        if width > 0 && height > 0 && (canvas.width() != width || canvas.height() != height) {
            canvas.set_width(width);
            canvas.set_height(height);
        }
        ratio
    }

    fn draw(&self) {
        let canvas = self.ctx.canvas().expect("canvas is null");
        let ratio = self.fit(&canvas);
        let width = canvas.width() as f64;
        let height = canvas.height() as f64;
        self.ctx.clear_rect(0.0, 0.0, width, height);
        self.ctx.set_line_width(0.9 * ratio);
        self.ctx.set_line_join("round");
        let frame = self.field.frame(width, height);
        let (ground, edge) = (self.paper, self.current_ink);
        let dark = ground.iter().sum::<f64>() < 384.0;
        let tint = self
            .current_tint
            .map(|c| if dark { lerp(c, 255.0, 0.38) } else { c });
        for strand in &frame.strands {
            let Some(&(first_x, first_y)) = strand.points.first() else {
                continue;
            };
            if strand
                .points
                .iter()
                .all(|(x, y)| (x - first_x).abs() + (y - first_y).abs() < 0.5)
            {
                continue;
            }
            self.ctx.begin_path();
            self.ctx.move_to(first_x, first_y);
            for (x, y) in &strand.points[1..] {
                self.ctx.line_to(*x, *y);
            }
            self.ctx.close_path();
            let filled = (strand.mix - 1.0).clamp(0.0, 1.0);
            let fill = [0, 1, 2].map(|c| lerp(ground[c], tint[c], filled));
            self.ctx.set_fill_style_str(&format!(
                "rgb({:.0} {:.0} {:.0})",
                fill[0], fill[1], fill[2]
            ));
            self.ctx.fill();
            let mix = strand.mix.min(1.0);
            let rgb = [0, 1, 2].map(|c| lerp(edge[c], tint[c], mix));
            let alpha = lerp(frame.alpha, 1.0, mix * 0.5);
            self.ctx.set_stroke_style_str(&format!(
                "rgb({:.0} {:.0} {:.0} / {alpha:.3})",
                rgb[0], rgb[1], rgb[2]
            ));
            self.ctx.stroke();
        }
    }
}

fn read_paper(canvas: &HtmlCanvasElement) -> Option<[f64; 3]> {
    let window = web_sys::window()?;
    let style = window.get_computed_style(canvas).ok()??;
    let value = style.get_property_value("--bg").ok()?;
    parse_hex(value.trim())
}

fn parse_hex(hex: &str) -> Option<[f64; 3]> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let channel =
        |range: std::ops::Range<usize>| u8::from_str_radix(&hex[range], 16).ok().map(f64::from);
    Some([channel(0..2)?, channel(2..4)?, channel(4..6)?])
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
