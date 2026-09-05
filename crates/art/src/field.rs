use std::f64::consts::TAU;

use content::{ArtConfig, ShapeFamily};

use crate::math::fbm;

pub const RINGS: usize = 168;
pub const RING_STEPS: usize = 72;
const LATTICE_COLUMNS: usize = 14;

#[derive(Clone, Copy)]
struct Ring {
    center: [f64; 3],
    a: [f64; 3],
    b: [f64; 3],
}

pub struct ProjectedRing {
    pub points: Vec<(f64, f64)>,
    pub depth: f64,
    pub mix: f64,
}

pub struct Field {
    current: ArtConfig,
    target: ArtConfig,
    weights: [f64; 4],
    seed_phase: f64,
    clock: f64,
}

impl Field {
    pub fn new(config: ArtConfig) -> Self {
        Self {
            weights: one_hot(config.shape),
            seed_phase: seed_phase(config.seed),
            current: config.clone(),
            target: config,
            clock: 0.0,
        }
    }

    pub fn transition_to(&mut self, target: ArtConfig) {
        self.target = target;
    }

    pub fn settled(&self) -> bool {
        let target = one_hot(self.target.shape);
        self.weights
            .iter()
            .zip(target)
            .all(|(weight, goal)| (weight - goal).abs() < 0.01)
            && (self.seed_phase - seed_phase(self.target.seed)).abs() < 0.01
            && (self.current.twist - self.target.twist).abs() < 0.01
            && (self.current.rotation_x - self.target.rotation_x).abs() < 0.01
            && (self.current.rotation_y - self.target.rotation_y).abs() < 0.01
    }

    pub fn step(&mut self, dt: f64, drift: bool) -> f64 {
        if drift {
            self.clock += dt;
        }
        let factor = 1.0 - (-4.2 * dt).exp();
        self.current = self.current.lerp(&self.target, factor);
        let target_weights = one_hot(self.target.shape);
        for (weight, target) in self.weights.iter_mut().zip(target_weights) {
            *weight += (target - *weight) * factor;
        }
        self.seed_phase += (seed_phase(self.target.seed) - self.seed_phase) * factor;
        factor
    }

    pub fn rings(&self, width: f64, height: f64) -> Vec<ProjectedRing> {
        let config = &self.current;
        let idle = self.clock * 0.05;
        let center = (width * 0.5, height * 0.5);
        let zoom = height.min(width) * 0.27 * config.scale;
        let rotation = (
            config.rotation_x + idle * 0.4,
            config.rotation_y - idle * 0.3,
        );
        let phase = config.phase + self.seed_phase + idle;
        (0..RINGS)
            .map(|index| {
                let s = index as f64 / RINGS as f64;
                let ring = self.blended_ring(config, index, s, phase);
                let mut depth_sum = 0.0;
                let points = (0..=RING_STEPS)
                    .map(|step| {
                        let t = step as f64 / RING_STEPS as f64 * TAU;
                        let (sin_t, cos_t) = t.sin_cos();
                        let point = [
                            ring.center[0] + ring.a[0] * cos_t + ring.b[0] * sin_t,
                            ring.center[1] + ring.a[1] * cos_t + ring.b[1] * sin_t,
                            ring.center[2] + ring.a[2] * cos_t + ring.b[2] * sin_t,
                        ];
                        let (px, py, depth) = project(point, rotation, zoom, center);
                        depth_sum += depth;
                        (px, py)
                    })
                    .collect();
                ProjectedRing {
                    points,
                    depth: depth_sum / (RING_STEPS + 1) as f64,
                    mix: smooth(((s * TAU + phase * 0.5).cos() * 0.5 + 0.5 - 0.3) / 0.5),
                }
            })
            .collect()
    }

    fn blended_ring(&self, config: &ArtConfig, index: usize, s: f64, phase: f64) -> Ring {
        let mut out = Ring {
            center: [0.0; 3],
            a: [0.0; 3],
            b: [0.0; 3],
        };
        for (family, weight) in FAMILIES.iter().zip(self.weights) {
            if weight < 0.002 {
                continue;
            }
            let ring = family_ring(*family, config, index, s, phase);
            for axis in 0..3 {
                out.center[axis] += ring.center[axis] * weight;
                out.a[axis] += ring.a[axis] * weight;
                out.b[axis] += ring.b[axis] * weight;
            }
        }
        let noise = config.warp * 0.35;
        if noise > 0.0 {
            let n = fbm(s * 4.0, phase * 0.2, 3, config.seed);
            out.center[2] += n * noise;
        }
        out
    }
}

const FAMILIES: [ShapeFamily; 4] = [
    ShapeFamily::Knot,
    ShapeFamily::Orbit,
    ShapeFamily::Lattice,
    ShapeFamily::Ribbon,
];

fn family_ring(family: ShapeFamily, c: &ArtConfig, index: usize, s: f64, phase: f64) -> Ring {
    let theta = s * TAU;
    let wave = (theta * c.symmetry + phase).sin() * c.warp;
    match family {
        ShapeFamily::Knot => {
            let radial = [theta.cos(), theta.sin(), 0.0];
            let up = [0.0, 0.0, 1.0];
            let loop_radius = c.radius * 0.72 * (1.0 + 0.1 * wave);
            let roll = c.twist * 0.5 * theta + phase * 0.3;
            let (sr, cr) = roll.sin_cos();
            let tube = 0.34 + c.secondary_radius * 0.45;
            Ring {
                center: scale(radial, loop_radius),
                a: scale(add(scale(radial, cr), scale(up, sr)), tube * 1.15),
                b: scale(add(scale(radial, -sr), scale(up, cr)), tube * 0.85),
            }
        }
        ShapeFamily::Orbit => {
            let turns = 1.0 + c.twist * 0.25;
            let angle = theta * turns + phase * 0.2;
            let swell = (s * std::f64::consts::PI).sin();
            let r = c.radius * (0.25 + 0.85 * swell);
            let tilt = c.secondary_radius * (theta * 2.0 + phase).sin();
            Ring {
                center: [
                    angle.cos() * c.radius * 0.18,
                    (s - 0.5) * c.radius * 2.0,
                    angle.sin() * c.radius * 0.18,
                ],
                a: [r * angle.cos(), tilt * 0.3, r * angle.sin()],
                b: [-r * angle.sin() * 0.45, 0.0, r * angle.cos() * 0.45],
            }
        }
        ShapeFamily::Lattice => {
            let rows = RINGS.div_ceil(LATTICE_COLUMNS);
            let gx = (index % LATTICE_COLUMNS) as f64 / (LATTICE_COLUMNS - 1) as f64 * 2.0 - 1.0;
            let gy = (index / LATTICE_COLUMNS) as f64 / (rows - 1) as f64 * 2.0 - 1.0;
            let z = ((gx * c.symmetry + phase).sin() + (gy * c.symmetry * 0.7 - phase).cos())
                * c.warp
                * 0.6;
            let r = c.secondary_radius * 0.55 * (1.0 + 0.4 * (gx * gy * 3.0 + phase).sin());
            let spin = c.twist * 0.3 * (gx + gy) + phase * 0.5;
            Ring {
                center: [gx * c.radius * 1.05, gy * c.radius * 0.8, z],
                a: [r * spin.cos(), r * spin.sin(), 0.0],
                b: [-r * spin.sin(), r * spin.cos(), r * 0.4],
            }
        }
        ShapeFamily::Ribbon => {
            let x = (s - 0.5) * c.radius * 2.4;
            let y = (theta * (c.symmetry * 0.5).max(1.0) + phase).sin() * c.radius * 0.35;
            let z = (theta + phase).cos() * c.radius * 0.3;
            let roll = c.twist * 0.6 * theta + phase;
            let r = c.secondary_radius * 2.3 * (0.55 + 0.45 * (s * std::f64::consts::PI).sin());
            Ring {
                center: [x, y, z],
                a: [0.0, r * roll.cos(), r * roll.sin()],
                b: [r * 0.22, -r * roll.sin() * 0.35, r * roll.cos() * 0.35],
            }
        }
    }
}

fn one_hot(shape: ShapeFamily) -> [f64; 4] {
    let mut weights = [0.0; 4];
    let index = FAMILIES
        .iter()
        .position(|family| *family == shape)
        .unwrap_or(0);
    weights[index] = 1.0;
    weights
}

fn seed_phase(seed: u64) -> f64 {
    (seed % 997) as f64 / 997.0 * TAU
}

fn smooth(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn scale(vector: [f64; 3], amount: f64) -> [f64; 3] {
    [vector[0] * amount, vector[1] * amount, vector[2] * amount]
}

fn project(
    [x, y, z]: [f64; 3],
    (rotation_x, rotation_y): (f64, f64),
    zoom: f64,
    (cx, cy): (f64, f64),
) -> (f64, f64, f64) {
    let (sin_x, cos_x) = rotation_x.sin_cos();
    let (sin_y, cos_y) = rotation_y.sin_cos();
    let y1 = y * cos_x - z * sin_x;
    let z1 = y * sin_x + z * cos_x;
    let x1 = x * cos_y + z1 * sin_y;
    let z2 = -x * sin_y + z1 * cos_y;
    let perspective = 3.8 / (3.8 + z2);
    (
        cx + x1 * zoom * perspective,
        cy + y1 * zoom * perspective,
        ((z2 + 2.0) / 4.0).clamp(0.0, 1.0),
    )
}
