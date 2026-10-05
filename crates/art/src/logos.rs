use std::f64::consts::{PI, TAU};

use crate::field::{
    Camera, Plan, Scene, Shape, X, Y, Z, arc_band, circle, ellipse, moved, plane, polar, rounded,
    square, trace,
};
use crate::math::smootherstep;

fn sway(t: f64, scale: f64) -> Camera {
    Camera {
        yaw: 0.5 * (t * 0.33).sin(),
        pitch: -0.05 + 0.14 * (t * 0.27).sin(),
        scale,
        focal: 3.6,
    }
}

fn thickness(plan: &Plan) -> usize {
    if plan.shapes >= 64 { 3 } else { 2 }
}

fn wave(t: f64, speed: f64, phase: f64) -> f64 {
    0.5 + 0.5 * (t * speed + phase).sin()
}

fn glyph_t(w: f64, h: f64, s: f64) -> Vec<[f64; 2]> {
    vec![
        [-w, h],
        [w, h],
        [w, h - s],
        [s / 2.0, h - s],
        [s / 2.0, -h],
        [-s / 2.0, -h],
        [-s / 2.0, h - s],
        [-w, h - s],
    ]
}

fn glyph_s(w: f64, h: f64, s: f64) -> Vec<[f64; 2]> {
    vec![
        [w, h],
        [-w, h],
        [-w, -s / 2.0],
        [w - s, -s / 2.0],
        [w - s, s - h],
        [-w, s - h],
        [-w, -h],
        [w, -h],
        [w, s / 2.0],
        [s - w, s / 2.0],
        [s - w, h - s],
        [w, h - s],
    ]
}

fn plus(l: f64, s: f64) -> Vec<[f64; 2]> {
    vec![
        [s, l],
        [-s, l],
        [-s, s],
        [-l, s],
        [-l, -s],
        [-s, -s],
        [-s, -l],
        [s, -l],
        [s, -s],
        [l, -s],
        [l, s],
        [s, s],
    ]
}

fn rust_letter() -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    let mut letter = vec![[-0.46, 0.40], [0.18, 0.40]];
    for k in 1..6 {
        let angle = PI * 0.5 - PI * k as f64 / 6.0;
        letter.push([0.18 + 0.19 * angle.cos(), 0.21 + 0.19 * angle.sin()]);
    }
    letter.extend([
        [0.18, 0.02],
        [0.30, -0.30],
        [0.44, -0.30],
        [0.44, -0.40],
        [0.16, -0.40],
        [0.06, -0.04],
        [-0.14, -0.04],
        [-0.14, -0.30],
        [-0.02, -0.30],
        [-0.02, -0.40],
        [-0.46, -0.40],
        [-0.46, -0.30],
        [-0.30, -0.30],
        [-0.30, 0.30],
        [-0.46, 0.30],
    ]);
    let mut counter = vec![[-0.14, 0.26], [0.14, 0.26]];
    for k in 1..4 {
        let angle = PI * 0.5 - PI * k as f64 / 4.0;
        counter.push([0.14 + 0.07 * angle.cos(), 0.19 + 0.07 * angle.sin()]);
    }
    counter.extend([[0.14, 0.12], [-0.14, 0.12]]);
    (letter, counter)
}

pub(crate) fn rust(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: 0.5 * (t * 0.33).sin(),
        pitch: -0.05 + 0.14 * (t * 0.27).sin(),
        scale: 0.98,
        focal: 3.6,
    });
    let spin = t * 0.2;
    let layers = if plan.shapes >= 64 { 3 } else { 2 };
    let spacing = 0.07;
    let front = -spacing * (layers - 1) as f64 * 0.5;
    let (inner, outer, tip) = (0.6, 0.76, 0.88);
    let teeth = 32;
    let segments = 8;
    let step = TAU / teeth as f64;
    let per = teeth / segments;
    for layer in 0..layers {
        let z = front + spacing * (layers - 1 - layer) as f64;
        for segment in 0..segments {
            let a0 = spin + segment as f64 * step * per as f64;
            let a1 = a0 + step * per as f64;
            let mut outline = vec![polar(inner, a0)];
            for tooth in 0..per {
                let center = a0 + (tooth as f64 + 0.5) * step;
                outline.extend([
                    polar(outer, center - step * 0.34),
                    polar(tip, center - step * 0.2),
                    polar(tip, center + step * 0.2),
                    polar(outer, center + step * 0.34),
                ]);
            }
            outline.push(polar(inner, a1));
            for k in 1..4 {
                outline.push(polar(inner, a1 - (a1 - a0) * k as f64 / 4.0));
            }
            scene.add_flat(&outline, z, 0.0);
        }
    }
    for k in 0..5 {
        let angle = spin + PI * 0.5 + k as f64 * TAU / 5.0;
        scene.add_flat(
            &[
                polar(inner, angle - 0.2),
                polar(inner, angle + 0.2),
                polar(0.45, angle),
            ],
            front - 0.004,
            0.0,
        );
        let [x, y] = polar(0.555, angle);
        let rivet: Vec<[f64; 2]> = (0..12)
            .map(|j| {
                let [cx, cy] = circle(j as f64 / 12.0, 0.03);
                [x + cx, y + cy]
            })
            .collect();
        scene.add_flat(&rivet, front - 0.008, 1.0);
    }
    let (letter, counter) = rust_letter();
    let pop = 0.06 + 0.06 * (0.5 + 0.5 * (t * 1.1).sin());
    for layer in 0..layers {
        let z = front - 0.02 - pop * layer as f64 / (layers - 1) as f64 - 0.03 * layer as f64;
        scene.add_flat(&letter, z, 0.0);
        scene.add_flat(&counter, z - 0.002, 0.0);
    }
    scene.finish(0.72)
}

pub(crate) fn typescript(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(sway(t, 0.95));
    let layers = thickness(plan);
    scene.extrude(&rounded(0.62, 0.62, 0.1), 0.0, layers, 0.07, 1.0);
    for (index, glyph, x) in [
        (0, glyph_t(0.13, 0.19, 0.075), 0.12),
        (1, glyph_s(0.13, 0.19, 0.075), 0.42),
    ] {
        let pop = 0.05 + 0.12 * smootherstep(wave(t, 1.1, -0.7 * index as f64));
        scene.extrude(&moved(&glyph, x, -0.32, 1.0), -0.02 - pop, 2, 0.035, 0.0);
    }
    scene.finish(0.72)
}

pub(crate) fn go(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(sway(t, 0.95));
    let layers = thickness(plan);
    let bob = 0.03 * (t * 2.0).sin();
    for side in [-1.0, 1.0] {
        scene.add_flat(
            &ellipse([side * 0.36, 0.5 + bob], 0.12, 0.12, 16),
            0.14,
            0.0,
        );
        scene.add_flat(
            &ellipse([side * 0.36, 0.5 + bob], 0.06, 0.06, 12),
            0.13,
            0.0,
        );
        scene.add_flat(
            &ellipse([side * 0.2, -0.66 + bob], 0.13, 0.06, 14),
            0.08,
            0.0,
        );
        scene.add_flat(
            &ellipse([side * 0.47, -0.06 + bob], 0.06, 0.13, 14),
            0.06,
            0.0,
        );
    }
    scene.extrude(
        &ellipse([0.0, -0.04 + bob], 0.46, 0.6, 36),
        0.0,
        layers,
        0.05,
        0.0,
    );
    let blink = (t * 0.25).rem_euclid(1.0);
    let open = if blink > 0.95 {
        (1.0 - ((blink - 0.95) / 0.05 * PI).sin()).max(0.08)
    } else {
        1.0
    };
    let look = [0.06 * (t * 0.7).cos(), 0.04 * (t * 1.1).sin()];
    for side in [-1.0, 1.0] {
        let center = [side * 0.19, 0.24 + bob];
        scene.add_flat(&ellipse(center, 0.17, 0.17 * open, 20), -0.03, 0.0);
        scene.add_flat(
            &ellipse(
                [center[0] + look[0], center[1] + look[1] * open],
                0.065,
                0.065 * open,
                14,
            ),
            -0.05,
            1.0,
        );
    }
    for side in [-1.0, 1.0] {
        scene.add_flat(
            &moved(&square(0.022, 0.04), side * 0.024, -0.085 + bob, 1.0),
            -0.04,
            0.0,
        );
    }
    scene.add_flat(&ellipse([0.0, 0.02 + bob], 0.15, 0.09, 18), -0.05, 0.0);
    scene.add_flat(&ellipse([0.0, 0.08 + bob], 0.06, 0.04, 12), -0.07, 0.0);
    scene.finish(0.72)
}

pub(crate) fn cpp(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(sway(t, 0.95));
    let layers = thickness(plan);
    let hexagon: Vec<[f64; 2]> = (0..6)
        .map(|k| polar(0.74, PI / 2.0 + k as f64 * TAU / 6.0))
        .collect();
    scene.extrude(&hexagon, 0.0, layers, 0.07, 0.0);
    let pop = 0.05 * wave(t, 1.1, 0.0);
    scene.extrude(
        &arc_band([-0.14, 0.0], 0.36, 0.21, 0.25 * PI, 1.75 * PI, 12),
        -0.08 - pop,
        2,
        0.04,
        0.0,
    );
    for (index, x) in [(0, 0.27), (1, 0.5)] {
        let spin = t * 1.4 + index as f64 * 0.8;
        scene.add_poly(
            &plus(0.085, 0.028),
            [x, 0.0, -0.14],
            [spin.cos(), 0.0, spin.sin()],
            Y,
            1.0,
        );
    }
    scene.finish(0.72)
}

pub(crate) fn c(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(sway(t, 0.95));
    let layers = thickness(plan);
    let corner = |k: usize| polar(0.74, PI / 2.0 + k as f64 * TAU / 6.0);
    let spread = 0.07 * wave(t, 0.8, 0.0);
    for face in [[5, 0, 1], [1, 2, 3], [3, 4, 5]] {
        let mut outline = vec![[0.0, 0.0]];
        outline.extend(face.iter().map(|k| corner(*k)));
        let middle = corner(face[1]);
        let reach = middle[0].hypot(middle[1]);
        let (dx, dy) = (middle[0] / reach * spread, middle[1] / reach * spread);
        scene.extrude(&moved(&outline, dx, dy, 1.0), 0.0, layers, 0.06, 0.0);
    }
    let pop = 0.05 * wave(t, 1.0, 1.0);
    scene.extrude(
        &arc_band([0.0, 0.0], 0.33, 0.19, 0.25 * PI, 1.75 * PI, 12),
        -0.1 - pop,
        2,
        0.04,
        1.0,
    );
    scene.finish(0.72)
}

pub(crate) fn java(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(Camera {
        yaw: t * 0.25,
        pitch: -0.42 + 0.06 * (t * 0.3).sin(),
        scale: 0.95,
        focal: 3.6,
    });
    let disc = |scene: &mut Scene, y: f64, radius: f64, mix: f64| {
        scene.add(|u| plane([0.0, y, 0.0], X, Z, circle(u, radius)), mix);
    };
    disc(&mut scene, -0.52, 0.62, 0.0);
    disc(&mut scene, -0.49, 0.5, 0.0);
    let rings = if plan.shapes >= 64 { 9 } else { 6 };
    for k in 0..rings {
        let f = k as f64 / (rings - 1) as f64;
        disc(&mut scene, -0.47 + 0.6 * f, 0.27 + 0.09 * f, 0.0);
    }
    disc(&mut scene, 0.135, 0.31, 1.0);
    let handle = arc_band([0.0, 0.0], 0.2, 0.13, -1.45, 1.45, 12);
    for z in [0.025, -0.025] {
        scene.add_free(&handle, [0.3, -0.16, z], X, Y, 0.0);
    }
    let facing = scene.facing();
    let steps = (plan.points / 2).max(4);
    for wisp in 0..2 {
        let offset = (wisp as f64 - 0.5) * 0.16;
        let curve = |s: f64| {
            [
                offset + 0.07 * (s * 5.0 - t * 1.6 + wisp as f64 * 2.0).sin(),
                0.22 + 0.62 * s,
            ]
        };
        let width = |s: f64| 0.04 * (1.0 - s) * smootherstep(s / 0.15).max(0.15);
        let mut outline = Vec::with_capacity(steps * 2);
        for k in 0..steps {
            let s = k as f64 / (steps - 1) as f64;
            let [x, y] = curve(s);
            outline.push([x - width(s), y]);
        }
        for k in (0..steps).rev() {
            let s = k as f64 / (steps - 1) as f64;
            let [x, y] = curve(s);
            outline.push([x + width(s), y]);
        }
        scene.add_free(&outline, [0.0, 0.0, 0.0], facing, Y, 0.0);
    }
    scene.finish(0.72)
}

struct Track {
    points: Vec<[f64; 2]>,
    lengths: Vec<f64>,
}

impl Track {
    fn new(points: Vec<[f64; 2]>) -> Self {
        let mut lengths = vec![0.0];
        for pair in points.windows(2) {
            let last = lengths[lengths.len() - 1];
            lengths.push(last + (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]));
        }
        Self { points, lengths }
    }

    fn length(&self) -> f64 {
        self.lengths[self.lengths.len() - 1]
    }

    fn at(&self, s: f64) -> [f64; 2] {
        let n = self.points.len();
        let k = self.lengths.partition_point(|l| *l < s).clamp(1, n - 1);
        let (a, b) = (self.points[k - 1], self.points[k]);
        let f = (s - self.lengths[k - 1]) / (self.lengths[k] - self.lengths[k - 1]).max(1e-9);
        [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]
    }
}

fn fillet(points: &[[f64; 2]], radius: f64) -> Vec<[f64; 2]> {
    let mut out = vec![points[0]];
    for k in 1..points.len() - 1 {
        let (a, v, b) = (points[k - 1], points[k], points[k + 1]);
        let toward = |p: [f64; 2]| {
            let d = [p[0] - v[0], p[1] - v[1]];
            let l = d[0].hypot(d[1]).max(1e-9);
            let r = radius.min(l * 0.5);
            [v[0] + d[0] / l * r, v[1] + d[1] / l * r]
        };
        let (p0, p2) = (toward(a), toward(b));
        for i in 0..=6 {
            let f = i as f64 / 6.0;
            let g = 1.0 - f;
            out.push([
                g * g * p0[0] + 2.0 * g * f * v[0] + f * f * p2[0],
                g * g * p0[1] + 2.0 * g * f * v[1] + f * f * p2[1],
            ]);
        }
    }
    out.push(points[points.len() - 1]);
    out
}

fn direction(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let d = [b[0] - a[0], b[1] - a[1]];
    let l = d[0].hypot(d[1]).max(1e-9);
    [d[0] / l, d[1] / l]
}

fn band(spine: &[[f64; 2]], half: f64) -> Vec<[f64; 2]> {
    let n = spine.len();
    let normals: Vec<[f64; 2]> = (0..n)
        .map(|k| {
            let d = direction(spine[k.saturating_sub(1)], spine[(k + 1).min(n - 1)]);
            [-d[1], d[0]]
        })
        .collect();
    let left = (0..n).map(|k| {
        [
            spine[k][0] + normals[k][0] * half,
            spine[k][1] + normals[k][1] * half,
        ]
    });
    let right = (0..n).rev().map(|k| {
        [
            spine[k][0] - normals[k][0] * half,
            spine[k][1] - normals[k][1] * half,
        ]
    });
    left.chain(right).collect()
}

fn rounded_outline(points: &[[f64; 2]], radius: f64) -> Vec<[f64; 2]> {
    let n = points.len();
    let mut ring: Vec<[f64; 2]> = Vec::with_capacity(n + 2);
    ring.push(points[n - 1]);
    ring.extend_from_slice(points);
    ring.push(points[0]);
    let mut out = fillet(&ring, radius);
    out.pop();
    out.remove(0);
    out
}

fn aligned(outline: &[[f64; 2]], to: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let area = |p: &[[f64; 2]]| {
        (0..p.len())
            .map(|k| {
                let (a, b) = (p[k], p[(k + 1) % p.len()]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
    };
    let mut outline = outline.to_vec();
    if area(&outline).signum() != area(to).signum() {
        outline.reverse();
    }
    let start = (0..outline.len())
        .min_by(|a, b| {
            let d = |k: usize| (outline[k][0] - to[0][0]).hypot(outline[k][1] - to[0][1]);
            d(*a).total_cmp(&d(*b))
        })
        .unwrap_or(0);
    outline.rotate_left(start);
    outline
}

fn blend(from: &[[f64; 2]], to: &[[f64; 2]], f: f64) -> Vec<[f64; 2]> {
    from.iter()
        .zip(to)
        .map(|(a, b)| [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f])
        .collect()
}

fn snake(scene: &mut Scene, plan: &Plan, t: f64, side: f64, mix: f64, lift: f64) {
    let flip = |p: &[f64; 2]| [p[0] * side, p[1] * side];
    let logo: Vec<[f64; 2]> = rounded_outline(
        &[
            [-0.26, 0.56],
            [0.26, 0.56],
            [0.26, 0.04],
            [-0.3, 0.04],
            [-0.3, -0.3],
            [-0.56, -0.3],
            [-0.56, 0.2],
            [-0.26, 0.2],
        ],
        0.07,
    )
    .iter()
    .map(flip)
    .collect();
    let logo = trace(&logo, plan.points);
    let pose = [[-0.43, -0.26], [-0.43, 0.12], [0.0, 0.12], [0.0, 0.5]];
    let approach = [[-2.7, -1.8], [-1.7, -1.35], [-1.0, -1.05], [-0.43, -0.7]];
    let body = Track::new(fillet(&pose, 0.14)).length();
    let track: Vec<[f64; 2]> = approach.iter().chain(pose.iter()).map(flip).collect();
    let track = Track::new(fillet(&track, 0.14));
    let end = track.length();
    let cycle = (t / 10.0).fract() * 10.0;
    let travel = if cycle < 3.0 {
        1.0 - (1.0 - cycle / 3.0).powi(3)
    } else if cycle < 8.6 {
        1.0
    } else {
        1.0 - smootherstep((cycle - 8.6) / 1.4)
    };
    let fuse = smootherstep((cycle - 3.0) / 0.7) * (1.0 - smootherstep((cycle - 7.9) / 0.6));
    let head = body + (end - body) * travel;
    let spine = |s: f64| {
        let base = track.at(s);
        let d = direction(track.at(s - 0.01), track.at(s + 0.01));
        let amplitude = 0.08 * smootherstep((end - body - s) / 0.45);
        let offset = amplitude * (s * 9.0).sin();
        [base[0] - d[1] * offset, base[1] + d[0] * offset]
    };
    let frame = |s: f64| {
        let c = spine(s);
        let d = direction(spine(s - 0.01), spine(s + 0.01));
        (c, d, [-d[1], d[0]])
    };
    let width = |back: f64| {
        let snout = 0.6 + 0.4 * (PI * 0.5 * (back / 0.07).min(1.0)).sin();
        let neck = 1.0 - 0.18 * (-((back - 0.2) / 0.05).powi(2)).exp();
        let tail = 1.0 - 0.85 * smootherstep((back / body - 0.72) / 0.28);
        0.12 * snout * neck * tail
    };
    let samples = 30;
    let mut left = Vec::with_capacity(samples + 1);
    let mut right = Vec::with_capacity(samples + 1);
    for k in 0..=samples {
        let back = body * (k as f64 / samples as f64).powf(1.3);
        let (c, _, n) = frame(head - back);
        let w = width(back);
        left.push([c[0] + n[0] * w, c[1] + n[1] * w]);
        right.push([c[0] - n[0] * w, c[1] - n[1] * w]);
    }
    let (c, d, n) = frame(head);
    let local = |x: f64, y: f64| [c[0] + d[0] * x + n[0] * y, c[1] + d[1] * x + n[1] * y];
    let mut outline = vec![local(0.035, -0.04), local(0.045, 0.0), local(0.035, 0.04)];
    outline.extend(left);
    outline.extend(right.into_iter().rev());
    let traced = trace(&outline, plan.points);
    let target = aligned(&logo, &traced);
    scene.add_part(&blend(&traced, &target, fuse), [0.0; 3], X, Y, mix, lift);
    let marks = 1.0 - smootherstep(fuse / 0.6);
    if marks > 0.05 {
        for k in 0..6 {
            let back = body * (0.24 + 0.1 * k as f64);
            let (m, md, mn) = frame(head - back);
            let size = 0.055 * marks * (1.0 - 0.6 * smootherstep((back / body - 0.72) / 0.28));
            let diamond = [
                [m[0] + md[0] * size * 1.3, m[1] + md[1] * size * 1.3],
                [m[0] + mn[0] * size, m[1] + mn[1] * size],
                [m[0] - md[0] * size * 1.3, m[1] - md[1] * size * 1.3],
                [m[0] - mn[0] * size, m[1] - mn[1] * size],
            ];
            scene.add_part(&diamond, [0.0; 3], X, Y, mix, lift + 0.001);
        }
    }
    let roaming = local(-0.08, 0.06);
    let resting = flip(&[-0.13, 0.43]);
    let eye = [
        roaming[0] + (resting[0] - roaming[0]) * fuse,
        roaming[1] + (resting[1] - roaming[1]) * fuse,
    ];
    let radius = 0.022 + 0.023 * fuse;
    scene.add_part(
        &ellipse(eye, radius, radius, 12),
        [0.0; 3],
        X,
        Y,
        if mix > 0.5 { 0.0 } else { 2.0 },
        lift + 0.002,
    );
}

pub(crate) fn python(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(sway(t, 0.95));
    snake(&mut scene, plan, t, 1.0, 0.0, 0.0);
    snake(&mut scene, plan, t - 0.35, -1.0, 1.0, 0.1);
    scene.finish(0.72)
}

fn letter_j() -> Vec<[f64; 2]> {
    let mut spine = vec![[0.1, -0.12], [0.1, -0.28]];
    for k in 0..=8 {
        let a = -PI * k as f64 / 8.0;
        spine.push([0.1 * a.cos(), -0.4 + 0.1 * a.sin()]);
    }
    band(&spine, 0.045)
}

fn letter_s() -> Vec<[f64; 2]> {
    let (cx, cy, rx, ry) = (0.38, -0.3325, 0.1, 0.084);
    let mut spine = Vec::new();
    for k in 0..=9 {
        let a = (30.0 + 240.0 * k as f64 / 9.0).to_radians();
        spine.push([cx + rx * a.cos(), cy + ry + ry * a.sin()]);
    }
    for k in 1..=9 {
        let a = (90.0 - 240.0 * k as f64 / 9.0).to_radians();
        spine.push([cx + rx * a.cos(), cy - ry + ry * a.sin()]);
    }
    band(&spine, 0.045)
}

pub(crate) fn javascript(plan: &Plan) -> Shape {
    let t = plan.t;
    let cycle = (t / 9.0).fract() * 9.0;
    let mut camera = sway(t, 0.95);
    camera.yaw += TAU * smootherstep((cycle - 7.0) / 1.6);
    let mut scene = plan.scene(camera);
    scene.extrude(&square(0.62, 0.62), 0.0, thickness(plan), 0.06, 1.0);
    for (index, glyph) in [letter_j(), letter_s()].into_iter().enumerate() {
        let pop = smootherstep((cycle - 0.3 - 0.35 * index as f64) / 0.45);
        let rise = 0.04 * (1.0 - pop);
        let lifted: Vec<[f64; 2]> = glyph.iter().map(|[x, y]| [*x, y - rise]).collect();
        if pop > 0.02 {
            scene.add_poly(&lifted, [0.0, 0.0, -0.012 - 0.1 * (1.0 - pop)], X, Y, 0.0);
        }
    }
    scene.finish(0.72)
}

pub(crate) fn dart(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(sway(t, 0.95));
    let at = |x: f64, y: f64| [(x - 213.0) * 1.3 / 355.0, (224.0 - y) * 1.3 / 355.0];
    let corner = at(95.0, 112.0);
    let pieces: [(Vec<[f64; 2]>, f64, f64); 5] = [
        (
            vec![
                corner,
                at(300.0, 112.0),
                at(312.0, 117.0),
                at(390.0, 195.0),
                at(390.0, 345.0),
                at(325.0, 345.0),
            ],
            0.0,
            1.0,
        ),
        (
            vec![
                corner,
                at(195.0, 57.0),
                at(210.0, 49.0),
                at(228.0, 49.0),
                at(243.0, 57.0),
                at(300.0, 112.0),
            ],
            -0.16,
            0.0,
        ),
        (vec![corner, at(325.0, 345.0), at(120.0, 345.0)], -0.08, 0.0),
        (
            vec![
                corner,
                at(120.0, 345.0),
                at(40.0, 250.0),
                at(35.0, 232.0),
                at(42.0, 214.0),
            ],
            0.12,
            1.0,
        ),
        (
            vec![
                at(120.0, 345.0),
                at(325.0, 345.0),
                at(325.0, 400.0),
                at(175.0, 400.0),
            ],
            -0.2,
            0.0,
        ),
    ];
    let cycle = (t / 7.0).fract() * 7.0;
    let apart = smootherstep((cycle - 0.4) / 1.1) * (1.0 - smootherstep((cycle - 2.4) / 0.9));
    for (outline, depth, mix) in &pieces {
        let centre = outline
            .iter()
            .fold([0.0, 0.0], |s, p| [s[0] + p[0], s[1] + p[1]]);
        let centre = [
            centre[0] / outline.len() as f64,
            centre[1] / outline.len() as f64,
        ];
        let push = direction([0.05, 0.0], centre);
        let reach = 0.1 * apart;
        scene.add_poly(
            outline,
            [push[0] * reach, push[1] * reach, depth * apart],
            X,
            Y,
            *mix,
        );
    }
    scene.finish(0.72)
}
