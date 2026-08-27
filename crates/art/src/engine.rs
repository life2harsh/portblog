use content::ArtConfig;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::math::{fbm, lerp, smootherstep};

#[derive(Debug, Clone)]
struct Transition {
    from: ArtConfig,
    to: ArtConfig,
    t: f64,
    duration: f64,
}

impl Transition {
    fn current(&self) -> ArtConfig {
        let ease = smootherstep(self.t);
        self.from.lerp(&self.to, ease)
    }

    fn advance(&mut self, dt: f64) -> bool {
        self.t = (self.t + dt / self.duration).min(1.0);
        self.t >= 1.0
    }
}

pub struct ArtEngine {
    ctx: CanvasRenderingContext2d,
    current: ArtConfig,
    transition: Option<Transition>,
    last_time: Option<f64>,
    clock: f64,
}

impl ArtEngine {
    pub fn new(canvas: &HtmlCanvasElement, config: ArtConfig) -> Self {
        let ctx = canvas
            .get_context("2d")
            .expect("getContext failed")
            .expect("2d context is null")
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("dyn_into failed");

        Self {
            ctx,
            current: config,
            transition: None,
            last_time: None,
            clock: 0.0,
        }
    }

    pub fn transition_to(&mut self, target: ArtConfig, duration_secs: f64) {
        let from = self
            .transition
            .as_ref()
            .map(|t| t.current())
            .unwrap_or_else(|| self.current.clone());

        self.transition = Some(Transition {
            from,
            to: target,
            t: 0.0,
            duration: duration_secs,
        });
    }

    pub fn set_config(&mut self, config: ArtConfig) {
        self.current = config;
        self.transition = None;
    }

    pub fn tick(&mut self, now_ms: f64) {
        let now_secs = now_ms / 1000.0;
        let dt = self.last_time.map_or(0.0, |t| (now_secs - t).min(0.1));
        self.last_time = Some(now_secs);
        self.clock += dt;

        if let Some(ref mut tr) = self.transition {
            let done = tr.advance(dt);
            if done {
                self.current = tr.to.clone();
                self.transition = None;
            }
        }

        let cfg = self
            .transition
            .as_ref()
            .map(|t| t.current())
            .unwrap_or_else(|| self.current.clone());

        self.draw(&cfg);
    }

    fn draw(&self, cfg: &ArtConfig) {
        let canvas = self.ctx.canvas().expect("canvas is null");
        let w = canvas.width() as f64;
        let h = canvas.height() as f64;

        self.ctx.clear_rect(0.0, 0.0, w, h);

        self.ctx.set_fill_style_str("rgb(250, 248, 243)");
        self.ctx.fill_rect(0.0, 0.0, w, h);

        let cx = w * 0.5;
        let cy = h * 0.5;
        let base_radius = h.min(w) * 0.35 * cfg.scale;
        let count = cfg.count.max(2) as usize;

        self.ctx.set_stroke_style_str("rgba(20, 18, 15, 0.55)");
        self.ctx.set_line_width(0.75);

        for i in 0..count {
            let t_norm = i as f64 / count as f64;
            let angle_base = t_norm * std::f64::consts::TAU + cfg.phase + self.clock * 0.12;

            let steps = 80usize;
            self.ctx.begin_path();

            for s in 0..=steps {
                let s_norm = s as f64 / steps as f64;
                let r = base_radius * (0.15 + s_norm * 0.85);
                let theta = angle_base
                    + cfg.twist * s_norm * std::f64::consts::TAU / count as f64
                    + cfg.frequency * (s_norm * std::f64::consts::PI).sin() * 0.4;

                let nx = (r / base_radius) * (angle_base / std::f64::consts::TAU);
                let ny = s_norm + self.clock * 0.04;
                let warp = fbm(nx * cfg.frequency, ny, 3, cfg.seed) * cfg.noise * base_radius * 0.5;

                let amp_mod = (cfg.amplitude * std::f64::consts::PI * s_norm * cfg.frequency * 3.0
                    + cfg.phase
                    + self.clock * 0.18)
                    .sin()
                    * base_radius
                    * cfg.amplitude
                    * 0.5;

                let px = cx + (r + amp_mod + warp) * theta.cos();
                let py = cy + (r + amp_mod + warp) * theta.sin();

                if s == 0 {
                    self.ctx.move_to(px, py);
                } else {
                    self.ctx.line_to(px, py);
                }
            }

            let alpha = lerp(0.08, 0.55, t_norm.powf(0.6));
            self.ctx.set_global_alpha(alpha);
            self.ctx.stroke();
        }

        self.ctx.set_global_alpha(1.0);

        let arm = 6.0;
        self.ctx.set_stroke_style_str("rgba(20, 18, 15, 0.25)");
        self.ctx.set_line_width(0.5);
        self.ctx.begin_path();
        self.ctx.move_to(cx - arm, cy);
        self.ctx.line_to(cx + arm, cy);
        self.ctx.move_to(cx, cy - arm);
        self.ctx.line_to(cx, cy + arm);
        self.ctx.stroke();
    }
}
