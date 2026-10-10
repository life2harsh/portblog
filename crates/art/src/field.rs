use std::f64::consts::{PI, TAU};

use content::{ArtConfig, Design};

use crate::math::{fbm, hash, smootherstep};

pub const SHAPES: usize = 96;
pub const POINTS: usize = 40;
const MORPH_SECONDS: f64 = 1.4;
const GOLDEN: f64 = 2.399_963_229_728_653;
const HIDDEN: f64 = 50.0;

pub struct Strand {
    pub points: Vec<(f64, f64)>,
    pub mix: f64,
}

pub struct Frame {
    pub strands: Vec<Strand>,
    pub alpha: f64,
}

pub(crate) struct Shape {
    points: Vec<[f64; 2]>,
    depth: Vec<f64>,
    mix: Vec<f64>,
    alpha: f64,
}

pub(crate) type V3 = [f64; 3];

pub(crate) const X: V3 = [1.0, 0.0, 0.0];
pub(crate) const Y: V3 = [0.0, 1.0, 0.0];
pub(crate) const Z: V3 = [0.0, 0.0, 1.0];

pub(crate) fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub(crate) fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(crate) fn mul(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

pub(crate) fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(crate) fn unit(a: V3) -> V3 {
    let length = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if length < 1e-9 {
        [1.0, 0.0, 0.0]
    } else {
        mul(a, 1.0 / length)
    }
}

pub(crate) fn frame_of(tangent: V3) -> (V3, V3) {
    let helper = if tangent[1].abs() > 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let side = unit(cross(tangent, helper));
    (side, cross(tangent, side))
}

pub(crate) fn plane(origin: V3, a: V3, b: V3, [u, v]: [f64; 2]) -> V3 {
    add(origin, add(mul(a, u), mul(b, v)))
}

pub(crate) fn circle(u: f64, radius: f64) -> [f64; 2] {
    let (sin, cos) = (u * TAU).sin_cos();
    [cos * radius, sin * radius]
}

pub(crate) fn rect(u: f64, half_width: f64, half_height: f64) -> [f64; 2] {
    let corners = [
        [half_width, -half_height],
        [half_width, half_height],
        [-half_width, half_height],
        [-half_width, -half_height],
    ];
    let walk = (u + 0.125).rem_euclid(1.0) * 4.0;
    let side = (walk as usize).min(3);
    let f = walk - side as f64;
    let (a, b) = (corners[side], corners[(side + 1) % 4]);
    [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]
}

pub(crate) fn lens(u: f64, half_length: f64, half_width: f64) -> [f64; 2] {
    let (sin, cos) = (u * TAU).sin_cos();
    [cos * half_length, sin * sin.abs() * half_width]
}

pub(crate) fn square(half_width: f64, half_height: f64) -> Vec<[f64; 2]> {
    vec![
        [half_width, half_height],
        [-half_width, half_height],
        [-half_width, -half_height],
        [half_width, -half_height],
    ]
}

pub(crate) fn ellipse(center: [f64; 2], rx: f64, ry: f64, count: usize) -> Vec<[f64; 2]> {
    (0..count)
        .map(|k| {
            let [x, y] = circle(k as f64 / count as f64, 1.0);
            [center[0] + x * rx, center[1] + y * ry]
        })
        .collect()
}

pub(crate) fn rounded(half_width: f64, half_height: f64, radius: f64) -> Vec<[f64; 2]> {
    let corners = [
        (half_width - radius, half_height - radius, 0.0),
        (radius - half_width, half_height - radius, 0.25),
        (radius - half_width, radius - half_height, 0.5),
        (half_width - radius, radius - half_height, 0.75),
    ];
    corners
        .iter()
        .flat_map(|(cx, cy, start)| {
            (0..4).map(move |k| {
                let angle = (start + 0.25 * k as f64 / 3.0) * TAU;
                [cx + radius * angle.cos(), cy + radius * angle.sin()]
            })
        })
        .collect()
}

pub(crate) fn arc_band(
    center: [f64; 2],
    outer: f64,
    inner: f64,
    from: f64,
    to: f64,
    steps: usize,
) -> Vec<[f64; 2]> {
    let at = |radius: f64, k: usize| {
        let [x, y] = polar(radius, from + (to - from) * k as f64 / steps as f64);
        [center[0] + x, center[1] + y]
    };
    (0..=steps)
        .map(|k| at(outer, k))
        .chain((0..=steps).rev().map(|k| at(inner, k)))
        .collect()
}

pub(crate) fn moved(vertices: &[[f64; 2]], dx: f64, dy: f64, k: f64) -> Vec<[f64; 2]> {
    vertices
        .iter()
        .map(|[x, y]| [x * k + dx, y * k + dy])
        .collect()
}

pub(crate) struct Camera {
    pub(crate) yaw: f64,
    pub(crate) pitch: f64,
    pub(crate) scale: f64,
    pub(crate) focal: f64,
}

impl Camera {
    pub(crate) fn project(&self, [x, y, z]: V3) -> ([f64; 2], f64) {
        let (sy, cy) = self.yaw.sin_cos();
        let (x1, z1) = (x * cy + z * sy, -x * sy + z * cy);
        let (sp, cp) = self.pitch.sin_cos();
        let (y2, z2) = (y * cp - z1 * sp, y * sp + z1 * cp);
        let s = self.focal / (self.focal + z2).max(0.2) * self.scale;
        ([x1 * s, -y2 * s], z2)
    }
}

pub(crate) struct Plan {
    pub(crate) shapes: usize,
    pub(crate) points: usize,
    pub(crate) seed: u64,
    pub(crate) t: f64,
}

impl Plan {
    pub(crate) fn unit(&self, k: u64) -> f64 {
        hash(self.seed.wrapping_mul(0x9e37_79b9).wrapping_add(k))
    }

    pub(crate) fn pick<T: Copy>(&self, k: u64, options: &[T]) -> T {
        options[((self.unit(k) * options.len() as f64) as usize).min(options.len() - 1)]
    }

    pub(crate) fn scene(&self, camera: Camera) -> Scene<'_> {
        Scene {
            plan: self,
            camera,
            points: Vec::with_capacity(self.shapes * self.points),
            depth: Vec::with_capacity(self.shapes),
            mix: Vec::with_capacity(self.shapes),
        }
    }
}

pub(crate) struct Scene<'a> {
    plan: &'a Plan,
    camera: Camera,
    points: Vec<[f64; 2]>,
    depth: Vec<f64>,
    mix: Vec<f64>,
}

impl Scene<'_> {
    pub(crate) fn add(&mut self, outline: impl Fn(f64) -> V3, mix: f64) {
        self.push(outline, mix, None);
    }

    pub(crate) fn add_flat(&mut self, vertices: &[[f64; 2]], z: f64, mix: f64) {
        self.add_poly(vertices, [0.0, 0.0, z], X, Y, mix);
    }

    pub(crate) fn add_poly(&mut self, vertices: &[[f64; 2]], origin: V3, a: V3, b: V3, mix: f64) {
        self.add_part(vertices, origin, a, b, mix, 0.0);
    }

    pub(crate) fn add_part(
        &mut self,
        vertices: &[[f64; 2]],
        origin: V3,
        a: V3,
        b: V3,
        mix: f64,
        lift: f64,
    ) {
        let depth = self.camera.project(origin).1 - lift;
        self.place(vertices, origin, a, b, mix, Some(depth));
    }

    pub(crate) fn add_free(&mut self, vertices: &[[f64; 2]], origin: V3, a: V3, b: V3, mix: f64) {
        self.place(vertices, origin, a, b, mix, None);
    }

    fn place(
        &mut self,
        vertices: &[[f64; 2]],
        origin: V3,
        a: V3,
        b: V3,
        mix: f64,
        depth: Option<f64>,
    ) {
        if vertices.len() < 2 {
            return;
        }
        let samples = trace(vertices, self.plan.points);
        let count = self.plan.points as f64;
        self.push(
            |u| {
                plane(
                    origin,
                    a,
                    b,
                    samples[((u * count).round() as usize).min(samples.len() - 1)],
                )
            },
            mix,
            depth,
        );
    }

    pub(crate) fn facing(&self) -> V3 {
        let (sin, cos) = self.camera.yaw.sin_cos();
        [cos, 0.0, sin]
    }

    pub(crate) fn extrude(
        &mut self,
        vertices: &[[f64; 2]],
        front: f64,
        layers: usize,
        spacing: f64,
        mix: f64,
    ) {
        for layer in (0..layers).rev() {
            let z = front + spacing * layer as f64;
            self.add_flat(vertices, z, if layer == 0 { mix } else { 0.0 });
        }
    }

    fn push(&mut self, outline: impl Fn(f64) -> V3, mix: f64, anchor: Option<f64>) {
        if self.depth.len() >= self.plan.shapes {
            return;
        }
        let mut depth = 0.0;
        for k in 0..self.plan.points {
            let (point, z) = self
                .camera
                .project(outline(k as f64 / self.plan.points as f64));
            self.points.push(point);
            depth += z;
        }
        self.depth
            .push(anchor.unwrap_or(depth / self.plan.points as f64));
        self.mix.push(mix);
    }

    pub(crate) fn finish(mut self, alpha: f64) -> Shape {
        while self.depth.len() < self.plan.shapes {
            self.points
                .extend(std::iter::repeat_n([0.0, 0.0], self.plan.points));
            self.depth.push(HIDDEN);
            self.mix.push(0.0);
        }
        Shape {
            points: self.points,
            depth: self.depth,
            mix: self.mix,
            alpha,
        }
    }
}

pub struct Field {
    shapes: usize,
    points: usize,
    target: ArtConfig,
    from: Option<Shape>,
    progress: f64,
    clock: f64,
}

impl Field {
    pub fn new(config: ArtConfig) -> Self {
        Self::with_resolution(config, SHAPES, POINTS)
    }

    pub fn with_resolution(config: ArtConfig, shapes: usize, points: usize) -> Self {
        Self {
            shapes: shapes.max(8),
            points: points.max(8),
            target: config,
            from: None,
            progress: 1.0,
            clock: 0.0,
        }
    }

    pub fn transition_to(&mut self, target: ArtConfig) {
        if target == self.target {
            return;
        }
        self.from = Some(self.shape());
        self.target = target;
        self.progress = 0.0;
    }

    pub fn settled(&self) -> bool {
        self.from.is_none()
    }

    pub fn step(&mut self, dt: f64, drift: bool) -> f64 {
        if drift {
            self.clock += dt;
        }
        if self.from.is_some() {
            self.progress += dt / MORPH_SECONDS;
            if self.progress >= 1.0 {
                self.progress = 1.0;
                self.from = None;
            }
        }
        1.0 - (-4.2 * dt).exp()
    }

    pub fn frame(&self, width: f64, height: f64) -> Frame {
        let unit = width.min(height) * 0.5 * 0.9;
        let shape = self.shape();
        let (cx, cy) = (width * 0.5, height * 0.5);
        let mut order: Vec<usize> = (0..self.shapes).collect();
        order.sort_by(|a, b| shape.depth[*b].total_cmp(&shape.depth[*a]));
        let strands = order
            .into_iter()
            .map(|index| Strand {
                points: shape.points[index * self.points..(index + 1) * self.points]
                    .iter()
                    .map(|[x, y]| (cx + x * unit, cy + y * unit))
                    .collect(),
                mix: shape.mix[index],
            })
            .collect();
        Frame {
            strands,
            alpha: shape.alpha,
        }
    }

    fn shape(&self) -> Shape {
        let plan = Plan {
            shapes: self.shapes,
            points: self.points,
            seed: self.target.seed,
            t: self.clock,
        };
        let live = generate(self.target.design, &plan);
        let Some(from) = &self.from else {
            return live;
        };
        let e = smootherstep(self.progress);
        let blend = |a: f64, b: f64| a + (b - a) * e;
        Shape {
            points: from
                .points
                .iter()
                .zip(&live.points)
                .map(|(a, b)| [blend(a[0], b[0]), blend(a[1], b[1])])
                .collect(),
            depth: from
                .depth
                .iter()
                .zip(&live.depth)
                .map(|(a, b)| blend(*a, *b))
                .collect(),
            mix: from
                .mix
                .iter()
                .zip(&live.mix)
                .map(|(a, b)| blend(*a, *b))
                .collect(),
            alpha: blend(from.alpha, live.alpha),
        }
    }
}

fn generate(design: Design, plan: &Plan) -> Shape {
    match design {
        Design::Burst => burst(plan),
        Design::Moth => moth(plan),
        Design::Worm => worm(plan),
        Design::Carousel => carousel(plan),
        Design::Tower => tower(plan),
        Design::Invader => invader(plan),
        Design::Ribbon => ribbon(plan),
        Design::Fan => fan(plan),
        Design::Orbit => orbit(plan),
        Design::Domino => domino(plan),
        Design::Tiles => tiles(plan),
        Design::Cube => cube(plan),
        Design::Helix => helix(plan),
        Design::Stacks => stacks(plan),
        Design::Rust => crate::logos::rust(plan),
        Design::TypeScript => crate::logos::typescript(plan),
        Design::JavaScript => crate::logos::javascript(plan),
        Design::Python => crate::logos::python(plan),
        Design::Go => crate::logos::go(plan),
        Design::Cpp => crate::logos::cpp(plan),
        Design::C => crate::logos::c(plan),
        Design::Java => crate::logos::java(plan),
        Design::Dart => crate::logos::dart(plan),
        Design::Fork => crate::connect::fork(plan),
        Design::Mesh => crate::connect::mesh(plan),
        Design::Knot => crate::connect::knot(plan),
    }
}

fn burst(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: t * 0.09 + plan.unit(0) * TAU,
        pitch: 0.35 + 0.1 * (t * 0.17).sin(),
        scale: 0.78,
        focal: 3.2,
    });
    let n = plan.shapes;
    for i in 0..n {
        let k = i as u64;
        let y = 1.0 - 2.0 * (i as f64 + 0.5) / n as f64;
        let ring = (1.0 - y * y).sqrt();
        let phi = i as f64 * GOLDEN + plan.unit(100 + k) * 0.8;
        let direction = unit([ring * phi.cos(), y, ring * phi.sin()]);
        let length = 0.5 + 0.65 * plan.unit(200 + k);
        let width = 0.028 + 0.04 * plan.unit(300 + k);
        let reach = 0.06 + 0.14 * (0.5 + 0.5 * (t * 0.55 + i as f64 * 0.7).sin());
        let center = mul(direction, reach + length * 0.5);
        let roll = unit([
            plan.unit(400 + k) - 0.5,
            plan.unit(500 + k) - 0.5,
            plan.unit(600 + k) - 0.5,
        ]);
        let side = unit(cross(direction, roll));
        let mix = if plan.unit(700 + k) < 0.07 { 1.0 } else { 0.0 };
        scene.add(
            |u| plane(center, direction, side, rect(u, length * 0.5, width)),
            mix,
        );
    }
    scene.finish(0.72)
}

fn moth(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: 0.35 * (t * 0.19).sin() + 0.2,
        pitch: -0.42 + 0.08 * (t * 0.23).sin(),
        scale: 1.15,
        focal: 3.4,
    });
    let bob = 0.04 * (t * 1.9).sin();
    let flap = 1.05 * (0.5 + 0.5 * (t * 1.9).sin()).powf(1.4);
    let segments = (plan.shapes / 10).max(4);
    let spots = 3usize;
    let petals = plan.shapes.saturating_sub(segments + spots * 2);
    let upper = petals * 3 / 10;
    let lower = petals / 2 - upper;
    let wing = |side: f64, a: f64, b: f64, lift: f64| {
        let (sf, cf) = flap.sin_cos();
        [side * a * cf, b + bob, a * sf + lift]
    };
    for (count, from, to, reach, root) in [
        (upper, 0.18, 1.35, 0.78, 0.08),
        (lower, -1.25, -0.2, 0.5, -0.02),
    ] {
        for side in [-1.0, 1.0] {
            for k in 0..count {
                let f = k as f64 / (count.max(2) - 1) as f64;
                let theta = from + (to - from) * f;
                let length = reach * (0.55 + 0.45 * (PI * f).sin()) * (1.0 - 0.12 * f);
                let (st, ct) = theta.sin_cos();
                let lift = -(k as f64) * 0.0015;
                let tint = if k % 7 == 3 { 1.0 } else { 0.0 };
                scene.add(
                    |u| {
                        let [x, y] = lens(u, length * 0.5, length * 0.13);
                        let along = length * 0.5 + x;
                        wing(side, along * ct - y * st, root + along * st + y * ct, lift)
                    },
                    tint,
                );
            }
        }
    }
    for side in [-1.0, 1.0] {
        for ring in 0..spots {
            let radius = 0.09 - ring as f64 * 0.028;
            scene.add(
                |u| {
                    let [x, y] = circle(u, radius);
                    wing(side, 0.42 + x, 0.36 + y, -0.06 - ring as f64 * 0.004)
                },
                if ring == spots - 1 { 1.0 } else { 0.0 },
            );
        }
    }
    for k in 0..segments {
        let f = k as f64 / (segments - 1) as f64;
        let y = 0.34 - 0.86 * f + bob;
        let radius = 0.035 + 0.04 * (PI * (0.15 + 0.85 * f)).sin();
        scene.add(
            |u| {
                plane(
                    [0.0, y, -0.12 - f * 0.01],
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    lens(u, radius, radius * 1.6),
                )
            },
            0.0,
        );
    }
    scene.finish(0.72)
}

fn worm(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: t * 0.1 + plan.unit(0) * TAU,
        pitch: 0.3,
        scale: 0.9,
        focal: 3.4,
    });
    let (a, b, c) = plan.pick(
        1,
        &[
            (1.0, 2.0, 1.0),
            (2.0, 1.0, 3.0),
            (1.0, 3.0, 2.0),
            (2.0, 3.0, 1.0),
        ],
    );
    let phase = [
        plan.unit(2) * TAU + t * 0.08,
        plan.unit(3) * TAU,
        plan.unit(4) * TAU - t * 0.06,
    ];
    let path = |s: f64| {
        [
            0.78 * (PI * a * s + phase[0]).sin(),
            0.42 * (PI * b * s + phase[1]).sin(),
            0.6 * (PI * c * s + phase[2]).sin(),
        ]
    };
    let n = plan.shapes;
    for i in 0..n {
        let s = i as f64 / (n - 1) as f64;
        let tangent = unit(sub(path(s + 0.002), path(s - 0.002)));
        let (side, up) = frame_of(tangent);
        let swell = (PI * s).sin().powf(0.7);
        let radius = 0.04 + 0.2 * swell * (0.78 + 0.22 * (s * 16.0 - t * 1.6).sin());
        let center = path(s);
        scene.add(
            |u| plane(center, side, up, circle(u, radius)),
            if i % 16 == 8 { 1.0 } else { 0.0 },
        );
    }
    scene.finish(0.72)
}

fn carousel(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: plan.unit(0) * TAU,
        pitch: 0.42,
        scale: 0.85,
        focal: 3.0,
    });
    let tiers = 3usize;
    for tier in 0..tiers {
        let count = plan.shapes / tiers + usize::from(tier < plan.shapes % tiers);
        let k = tier as f64 - 1.0;
        let y = k * 0.56;
        let radius = 0.8 - k.abs() * 0.14;
        let spin = t * 0.14 * if tier % 2 == 0 { 1.0 } else { -1.0 } + tier as f64 * 0.4;
        let lean = 0.25 * k;
        for i in 0..count {
            let phi = TAU * i as f64 / count as f64 + spin;
            let (sp, cp) = phi.sin_cos();
            let center = [radius * cp, y, radius * sp];
            let tangent = [-sp, 0.0, cp];
            let up = unit([cp * lean, 1.0, sp * lean]);
            scene.add(
                |u| plane(center, tangent, up, rect(u, 0.12, 0.21)),
                if i == 0 { 1.0 } else { 0.0 },
            );
        }
    }
    scene.finish(0.72)
}

fn tower(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: t * 0.12 + plan.unit(0) * TAU,
        pitch: 0.5,
        scale: 0.9,
        focal: 3.4,
    });
    let n = plan.shapes;
    let twist = 2.4 + 0.8 * (t * 0.23).sin();
    let profile = plan.unit(1) * TAU;
    for i in 0..n {
        let v = i as f64 / (n - 1) as f64;
        let y = -0.82 + 1.64 * v;
        let size = 0.2 + 0.2 * (0.5 + 0.5 * (TAU * v * 1.3 + profile).cos());
        let angle = v * twist + t * 0.2;
        let (sa, ca) = angle.sin_cos();
        let a = [ca, 0.0, sa];
        let b = [-sa, 0.0, ca];
        scene.add(
            |u| plane([0.0, y, 0.0], a, b, rect(u, size, size)),
            if i % 12 == 6 { 1.0 } else { 0.0 },
        );
    }
    scene.finish(0.72)
}

const INVADER: [[&str; 8]; 2] = [
    [
        "..X.....X..",
        "...X...X...",
        "..XXXXXXX..",
        ".XX.XXX.XX.",
        "XXXXXXXXXXX",
        "X.XXXXXXX.X",
        "X.X.....X.X",
        "...XX.XX...",
    ],
    [
        "..X.....X..",
        "X..X...X..X",
        "X.XXXXXXX.X",
        "XXX.XXX.XXX",
        "XXXXXXXXXXX",
        ".XXXXXXXXX.",
        "..X.....X..",
        ".X.......X.",
    ],
];

fn invader(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: 0.55 * (t * 0.31).sin(),
        pitch: -0.18 + 0.14 * (t * 0.23).sin(),
        scale: 1.0,
        focal: 3.4,
    });
    let sprite = INVADER[((t * 1.4) as usize) % 2];
    let pixels: Vec<(usize, usize)> = sprite
        .iter()
        .enumerate()
        .flat_map(|(row, line)| {
            line.bytes()
                .enumerate()
                .filter(|(_, b)| *b == b'X')
                .map(move |(column, _)| (row, column))
        })
        .collect();
    let layers = (plan.shapes / 48).clamp(1, 2);
    let pitch = 1.62 / 11.0;
    let hover = 0.05 * (t * 1.1).sin();
    for column in [3usize, 7] {
        let x = (column as f64 - 5.0) * pitch;
        let y = 0.5 * pitch + hover;
        let z = 0.07 * (t * 2.2 - column as f64 * 0.7 - 1.2).sin() - 0.02;
        let size = pitch * 0.4;
        scene.add(
            |u| {
                plane(
                    [x, y, z],
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    rect(u, size, size),
                )
            },
            2.0,
        );
    }
    for layer in 0..layers {
        for (row, column) in &pixels {
            let x = (*column as f64 - 5.0) * pitch;
            let y = (3.5 - *row as f64) * pitch + hover;
            let ripple = 0.07 * (t * 2.2 - *column as f64 * 0.7 - *row as f64 * 0.4).sin();
            let z = layer as f64 * 0.16 + ripple;
            let size = pitch * if layer == 0 { 0.44 } else { 0.4 };
            scene.add(
                |u| {
                    plane(
                        [x, y, z],
                        [1.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0],
                        rect(u, size, size),
                    )
                },
                0.0,
            );
        }
    }
    scene.finish(0.72)
}

fn ribbon(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: t * 0.08 + plan.unit(0) * TAU,
        pitch: 0.55 + 0.2 * (t * 0.13).sin(),
        scale: 0.64,
        focal: 4.0,
    });
    let (p, q) = plan.pick(1, &[(2.0, 3.0), (3.0, 2.0), (2.0, 5.0)]);
    let knot = |s: f64| {
        let theta = TAU * s;
        let r = 1.0 + 0.42 * (q * theta).cos();
        [
            r * (p * theta).cos(),
            0.42 * (q * theta).sin(),
            r * (p * theta).sin(),
        ]
    };
    let n = plan.shapes;
    for i in 0..n {
        let s = i as f64 / n as f64;
        let center = knot(s);
        let tangent = unit(sub(knot(s + 0.001), knot(s - 0.001)));
        let (side, up) = frame_of(tangent);
        let roll = s * TAU * 3.0 + t * 0.6;
        let across = add(mul(side, roll.cos()), mul(up, roll.sin()));
        scene.add(
            |u| plane(center, across, tangent, rect(u, 0.22, 0.075)),
            if i % 12 == 0 { 1.0 } else { 0.0 },
        );
    }
    scene.finish(0.72)
}

fn fan(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: 0.35 * (t * 0.21).sin(),
        pitch: 0.18,
        scale: 0.95,
        focal: 3.6,
    });
    let n = plan.shapes - 1;
    let spread = 1.1 + 0.9 * (0.5 + 0.5 * (t * 0.33).sin());
    let pivot = [0.0, -0.72, 0.0];
    for i in 0..n {
        let f = i as f64 / (n - 1) as f64 - 0.5;
        let angle = f * spread;
        let length = 1.3 + 0.12 * (i as f64 * 0.45 + t * 0.8).sin();
        let direction = [angle.sin(), angle.cos(), 0.0];
        let across = [angle.cos(), -angle.sin(), 0.0];
        let center = add(
            add(pivot, mul(direction, length * 0.5)),
            [0.0, 0.0, -(i as f64) * 0.004],
        );
        scene.add(
            |u| plane(center, direction, across, rect(u, length * 0.5, 0.1)),
            if i % 11 == 5 { 1.0 } else { 0.0 },
        );
    }
    scene.add(
        |u| {
            plane(
                add(pivot, [0.0, 0.0, -0.5]),
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                circle(u, 0.07),
            )
        },
        1.0,
    );
    scene.finish(0.72)
}

fn orbit(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: t * 0.05 + plan.unit(0) * TAU,
        pitch: 0.62,
        scale: 0.9,
        focal: 3.4,
    });
    let core = 0.3;
    let slices = 11usize;
    for i in 0..slices {
        let y = (i as f64 / (slices - 1) as f64 * 2.0 - 1.0) * core * 0.92;
        let radius = (core * core - y * y).max(0.0).sqrt();
        scene.add(
            |u| {
                plane(
                    [0.0, y, 0.0],
                    [1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0],
                    circle(u, radius),
                )
            },
            0.0,
        );
    }
    let orbits = 4usize;
    let beads = plan.shapes.saturating_sub(slices);
    for k in 0..orbits {
        let count = beads / orbits + usize::from(k < beads % orbits);
        let tilt = unit([
            plan.unit(10 + k as u64) - 0.5,
            1.4,
            plan.unit(20 + k as u64) - 0.5,
        ]);
        let (e1, e2) = frame_of(tilt);
        let reach = 0.56 + 0.15 * k as f64;
        let speed = 0.5 / reach.powf(1.5) * if k % 2 == 0 { 1.0 } else { -1.0 };
        let start = plan.unit(30 + k as u64) * TAU;
        for j in 0..count {
            let angle = start + t * speed - j as f64 * 0.11 * speed.signum();
            let radial = add(mul(e1, angle.cos()), mul(e2, angle.sin()));
            let center = mul(radial, reach);
            let size = 0.015 + 0.1 * (1.0 - j as f64 / count as f64).powf(0.8);
            scene.add(
                |u| plane(center, radial, tilt, circle(u, size)),
                if k == 1 && j == 0 { 1.0 } else { 0.0 },
            );
        }
    }
    scene.finish(0.72)
}

fn domino(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: 0.6 + t * 0.05 + plan.unit(0) * TAU,
        pitch: 0.72,
        scale: 0.92,
        focal: 3.4,
    });
    let n = plan.shapes;
    let turns = 2.1;
    let spiral = |s: f64| {
        let theta = s * turns * TAU;
        let r = 0.14 + 0.78 * s;
        [r * theta.cos(), 0.0, r * theta.sin()]
    };
    for i in 0..n {
        let s = (i as f64 / (n - 1) as f64).sqrt();
        let base = spiral(s);
        let tangent = unit(sub(spiral(s + 0.001), spiral(s - 0.001)));
        let side = unit(cross([0.0, 1.0, 0.0], tangent));
        let phase = (t * 0.3 - s * 1.4).rem_euclid(1.0);
        let fall = (PI * (phase * 2.6).min(1.0)).sin().powi(2) * 1.05;
        let up = add(mul([0.0, 1.0, 0.0], fall.cos()), mul(tangent, fall.sin()));
        let height = 0.12 + 0.12 * s;
        let center = add(base, mul(up, height));
        scene.add(
            |u| plane(center, side, up, rect(u, 0.045 + 0.04 * s, height)),
            if i % 13 == 6 { 1.0 } else { 0.0 },
        );
    }
    scene.finish(0.72)
}

fn tiles(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: 0.7 + plan.unit(0) * 0.4,
        pitch: 0.62,
        scale: 0.95,
        focal: 3.0,
    });
    let side = (plan.shapes as f64).sqrt().floor().max(2.0) as usize;
    let gap = 1.9 / side as f64;
    let offset = plan.unit(1) * 30.0;
    let height = |x: f64, z: f64| {
        0.16 * (x * 3.2 + t * 1.1).sin() * (z * 2.4 - t * 0.7).cos()
            + 0.12 * fbm(x * 1.4 + offset, z * 1.4 + t * 0.12, 2, plan.seed)
    };
    for r in 0..side {
        for c in 0..side {
            let x = (c as f64 - (side - 1) as f64 * 0.5) * gap;
            let z = (r as f64 - (side - 1) as f64 * 0.5) * gap;
            let y = height(x, z);
            let dx = (height(x + 0.01, z) - height(x - 0.01, z)) / 0.02;
            let dz = (height(x, z + 0.01) - height(x, z - 0.01)) / 0.02;
            let a = unit([1.0, dx, 0.0]);
            let b = unit([0.0, dz, 1.0]);
            scene.add(
                |u| plane([x, y - 0.1, z], a, b, rect(u, gap * 0.45, gap * 0.45)),
                if (r * 7 + c * 3) % 17 == 0 { 1.0 } else { 0.0 },
            );
        }
    }
    scene.finish(0.72)
}

fn cube(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: t * 0.15 + plan.unit(0) * TAU,
        pitch: 0.5 + 0.12 * (t * 0.2).sin(),
        scale: 0.92,
        focal: 3.4,
    });
    let grid = ((plan.shapes / 6) as f64).sqrt().floor().max(1.0) as usize;
    let half = 0.46;
    let cell = 2.0 * half / grid as f64;
    let faces: [(V3, V3, V3); 6] = [
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
    ];
    let corner = [half, half, half];
    for (f, (normal, a, b)) in faces.iter().enumerate() {
        for i in 0..grid {
            for j in 0..grid {
                let offset = |k: usize| (k as f64 + 0.5) * cell - half;
                let rest = plane(mul(*normal, half), *a, *b, [offset(i), offset(j)]);
                let d = sub(rest, corner);
                let distance = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                let wave = (t * 1.1 - distance * 3.2).sin().max(0.0).powi(3);
                let center = add(rest, mul(*normal, 0.28 * wave));
                let size = cell * (0.46 - 0.1 * wave);
                scene.add(
                    |u| plane(center, *a, *b, rect(u, size, size)),
                    if (f * 5 + i * 3 + j) % 11 == 0 {
                        1.0
                    } else {
                        0.0
                    },
                );
            }
        }
    }
    scene.finish(0.72)
}

fn helix(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: t * 0.12 + plan.unit(0) * TAU,
        pitch: -0.22,
        scale: 0.92,
        focal: 3.4,
    });
    let levels = (plan.shapes / 3).max(2);
    let turns = 1.6 + 0.2 * (t * 0.17).sin();
    for i in 0..levels {
        let v = i as f64 / (levels - 1) as f64;
        let y = -0.9 + 1.8 * v;
        let angle = v * TAU * turns + t * 0.35;
        let (sa, ca) = angle.sin_cos();
        let radial = [ca, 0.0, sa];
        let tangent = [-sa, 0.0, ca];
        let reach = 0.48;
        let tint = if i % 6 == 3 { 1.0 } else { 0.0 };
        scene.add(
            |u| plane([0.0, y, 0.0], radial, tangent, rect(u, reach, 0.032)),
            tint,
        );
        for end in [-1.0, 1.0] {
            let center = add(mul(radial, reach * end), [0.0, y, 0.0]);
            scene.add(
                |u| plane(center, tangent, [0.0, 1.0, 0.0], circle(u, 0.06)),
                tint,
            );
        }
    }
    scene.finish(0.72)
}

fn stacks(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: 0.6 + t * 0.06 + plan.unit(0) * TAU,
        pitch: -0.55,
        scale: 0.95,
        focal: 3.2,
    });
    let side = 3usize;
    let columns = side * side;
    let per = (plan.shapes / columns).max(1);
    let gap = 0.56;
    for c in 0..columns {
        let (cx, cz) = (
            (c % side) as f64 - (side - 1) as f64 * 0.5,
            (c / side) as f64 - (side - 1) as f64 * 0.5,
        );
        let phase = plan.unit(40 + c as u64) * TAU;
        let rise = 0.25 + 0.75 * (0.5 + 0.5 * (t * 0.9 + phase).sin());
        let spacing = 0.9 * rise / per as f64;
        for k in 0..per {
            let y = -0.45 + k as f64 * spacing;
            let radius = 0.19 - 0.02 * (k as f64 / per as f64);
            let center = [cx * gap, y, cz * gap];
            scene.add(
                |u| plane(center, [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], circle(u, radius)),
                if c == 4 && k + 1 == per { 1.0 } else { 0.0 },
            );
        }
    }
    scene.finish(0.72)
}

pub(crate) fn trace(vertices: &[[f64; 2]], count: usize) -> Vec<[f64; 2]> {
    let n = vertices.len();
    let lerp =
        |a: [f64; 2], b: [f64; 2], f: f64| [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
    let lengths: Vec<f64> = (0..n)
        .map(|i| {
            let (a, b) = (vertices[i], vertices[(i + 1) % n]);
            (b[0] - a[0]).hypot(b[1] - a[1])
        })
        .collect();
    let total = lengths.iter().sum::<f64>().max(1e-9);
    if n > count {
        let mut out = Vec::with_capacity(count);
        let (mut edge, mut start) = (0, 0.0);
        for k in 0..count {
            let target = total * k as f64 / count as f64;
            while edge + 1 < n && start + lengths[edge] < target {
                start += lengths[edge];
                edge += 1;
            }
            let f = ((target - start) / lengths[edge].max(1e-9)).clamp(0.0, 1.0);
            out.push(lerp(vertices[edge], vertices[(edge + 1) % n], f));
        }
        return out;
    }
    let spare = count - n;
    let share: Vec<f64> = lengths.iter().map(|l| l / total * spare as f64).collect();
    let mut extra: Vec<usize> = share.iter().map(|s| *s as usize).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|a, b| (share[*b] - extra[*b] as f64).total_cmp(&(share[*a] - extra[*a] as f64)));
    let left = spare - extra.iter().sum::<usize>();
    for index in order.into_iter().take(left) {
        extra[index] += 1;
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..n {
        out.push(vertices[i]);
        for j in 1..=extra[i] {
            out.push(lerp(
                vertices[i],
                vertices[(i + 1) % n],
                j as f64 / (extra[i] + 1) as f64,
            ));
        }
    }
    out
}

pub(crate) fn polar(radius: f64, angle: f64) -> [f64; 2] {
    let (sin, cos) = angle.sin_cos();
    [cos * radius, sin * radius]
}

pub fn raster(frame: &Frame, width: usize, height: usize) -> Vec<u8> {
    let mut grid = vec![0u8; width * height];
    let mut crossings = Vec::new();
    for strand in &frame.strands {
        let points = &strand.points;
        if points.len() < 3 {
            continue;
        }
        let (first_x, first_y) = points[0];
        if points
            .iter()
            .all(|(x, y)| (x - first_x).abs() + (y - first_y).abs() < 0.01)
        {
            continue;
        }
        let fill = if strand.mix > 1.5 { 2 } else { 0 };
        let top = points.iter().map(|p| p.1).fold(f64::MAX, f64::min).max(0.0) as usize;
        let bottom =
            (points.iter().map(|p| p.1).fold(f64::MIN, f64::max).ceil() as usize).min(height);
        for row in top..bottom {
            let y = row as f64 + 0.5;
            crossings.clear();
            for k in 0..points.len() {
                let (a, b) = (points[k], points[(k + 1) % points.len()]);
                if (a.1 <= y) != (b.1 <= y) {
                    crossings.push(a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            crossings.sort_by(f64::total_cmp);
            for pair in crossings.chunks(2) {
                if let [from, to] = pair {
                    let start = (from.max(0.0).round() as usize).min(width);
                    let end = (to.max(0.0).round() as usize).min(width);
                    for cell in &mut grid[row * width + start..row * width + end.max(start)] {
                        *cell = fill;
                    }
                }
            }
        }
        let ink = if strand.mix > 0.5 { 2 } else { 1 };
        for k in 0..points.len() {
            let (a, b) = (points[k], points[(k + 1) % points.len()]);
            let steps = ((b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil() as usize).max(1);
            for s in 0..=steps {
                let f = s as f64 / steps as f64;
                let x = a.0 + (b.0 - a.0) * f;
                let y = a.1 + (b.1 - a.1) * f;
                if x >= 0.0 && y >= 0.0 && (x as usize) < width && (y as usize) < height {
                    grid[y as usize * width + x as usize] = ink;
                }
            }
        }
    }
    grid
}
