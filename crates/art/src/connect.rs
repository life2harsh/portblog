use std::f64::consts::{PI, TAU};

use crate::field::{Camera, Plan, Shape, V3, X, Y, Z, add, cross, ellipse, mul, square, sub, unit};

fn spin(p: V3, angle: f64) -> V3 {
    let (sin, cos) = angle.sin_cos();
    [p[0] * cos + p[2] * sin, p[1], p[2] * cos - p[0] * sin]
}

fn tilt(p: V3, angle: f64) -> V3 {
    let (sin, cos) = angle.sin_cos();
    [p[0], p[1] * cos - p[2] * sin, p[1] * sin + p[2] * cos]
}

fn across(direction: V3) -> V3 {
    let side = cross(direction, Z);
    if side.iter().map(|c| c * c).sum::<f64>() < 1e-6 {
        X
    } else {
        unit(side)
    }
}

fn camera(t: f64) -> Camera {
    Camera {
        yaw: 0.25 * (t * 0.21).sin(),
        pitch: -0.08,
        scale: 0.95,
        focal: 3.6,
    }
}

const OCTOCAT: [[f64; 2]; 38] = [
    [-0.316, -0.949],
    [-0.249, -0.715],
    [-0.504, -0.695],
    [-0.585, -0.597],
    [-0.688, -0.456],
    [-0.689, -0.390],
    [-0.535, -0.492],
    [-0.377, -0.600],
    [-0.244, -0.575],
    [-0.180, -0.441],
    [-0.313, -0.417],
    [-0.436, -0.367],
    [-0.539, -0.279],
    [-0.609, -0.143],
    [-0.635, 0.053],
    [-0.608, 0.201],
    [-0.532, 0.321],
    [-0.552, 0.412],
    [-0.522, 0.586],
    [-0.457, 0.583],
    [-0.247, 0.484],
    [0.002, 0.518],
    [0.252, 0.484],
    [0.462, 0.584],
    [0.527, 0.586],
    [0.557, 0.412],
    [0.537, 0.321],
    [0.613, 0.202],
    [0.640, 0.052],
    [0.614, -0.144],
    [0.543, -0.280],
    [0.440, -0.367],
    [0.317, -0.417],
    [0.184, -0.441],
    [0.251, -0.626],
    [0.251, -0.801],
    [0.319, -0.949],
    [0.000, -1.000],
];

fn swing(point: [f64; 2], pivot: [f64; 2], angle: f64) -> [f64; 2] {
    let (sin, cos) = angle.sin_cos();
    let (x, y) = (point[0] - pivot[0], point[1] - pivot[1]);
    [pivot[0] + x * cos - y * sin, pivot[1] + x * sin + y * cos]
}

fn pulse(t: f64, every: f64, at: f64, span: f64) -> f64 {
    let f = ((t / every).fract() * every - at) / span;
    if (0.0..1.0).contains(&f) { f } else { -1.0 }
}

pub(crate) fn octocat(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(camera(t));
    let size = 0.9;
    let layers = if plan.shapes >= 64 { 3 } else { 2 };
    scene.extrude(&ellipse([0.0, 0.0], size, size, 40), 0.0, layers, 0.06, 0.0);

    let wag = 0.3 * (0.5 + 0.5 * (t * 2.4).sin());
    let tail = [-0.25, -0.64];
    let flick = pulse(t, 5.3, 2.0, 0.45);
    let twitch = if flick < 0.0 {
        0.0
    } else {
        0.2 * (PI * flick).sin() * (3.0 * PI * flick).sin()
    };
    let cat: Vec<[f64; 2]> = OCTOCAT
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let point = match index {
                2..=7 => {
                    let d = (point[0] - tail[0]).hypot(point[1] - tail[1]);
                    swing(*point, tail, wag * (d / 0.45).min(1.0))
                }
                17..=19 => swing(*point, [-0.42, 0.36], twitch),
                _ => *point,
            };
            [point[0] * size, point[1] * size]
        })
        .collect();
    scene.add_flat(&cat, -0.03, 2.0);

    let blink = pulse(t, 3.7, 3.45, 0.22);
    let open = if blink < 0.0 {
        1.0
    } else {
        1.0 - 0.9 * (PI * blink).sin()
    };
    let gaze = 0.03 * (t * 0.21).sin();
    for side in [-1.0, 1.0] {
        scene.add_flat(
            &ellipse(
                [(side * 0.21 + gaze) * size, 0.12 * size],
                0.07 * size,
                0.1 * size * open,
                16,
            ),
            -0.05,
            0.0,
        );
    }
    scene.finish(0.72)
}

pub(crate) fn mesh(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(camera(t));
    let count = if plan.shapes >= 64 { 32 } else { 18 };
    let golden = PI * (3.0 - 5f64.sqrt());
    let rest: Vec<V3> = (0..count)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f64 + 0.5) / count as f64;
            let r = (1.0 - y * y).sqrt();
            let a = golden * i as f64;
            [r * a.cos(), y, r * a.sin()]
        })
        .collect();
    let mut links: Vec<(usize, usize)> = Vec::new();
    for (i, p) in rest.iter().enumerate() {
        let mut near: Vec<(f64, usize)> = rest
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(j, q)| {
                let d = sub(*p, *q);
                (d[0] * d[0] + d[1] * d[1] + d[2] * d[2], j)
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, j) in near.into_iter().take(2) {
            let pair = (i.min(j), i.max(j));
            if !links.contains(&pair) {
                links.push(pair);
            }
        }
    }
    links.truncate(plan.shapes.saturating_sub(count));
    let breathe = 0.62 + 0.03 * (t * 1.3).sin();
    let placed: Vec<V3> = rest
        .iter()
        .map(|p| tilt(spin(mul(*p, breathe), t * 0.35), 0.35))
        .collect();
    for (k, (i, j)) in links.iter().enumerate() {
        let (a, b) = (placed[*i], placed[*j]);
        let axis = unit(sub(b, a));
        let d = sub(b, a);
        let span = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let w = 0.007;
        let signal = (k as f64 * 0.618 + t * 0.4).fract() < 0.14;
        scene.add_free(
            &[[0.0, -w], [span, -w], [span, w], [0.0, w]],
            a,
            axis,
            across(axis),
            if signal { 1.0 } else { 0.0 },
        );
    }
    for (i, p) in placed.iter().enumerate() {
        let hub = (i as f64 * 0.382 + t * 0.25).fract() < 0.1;
        let r = if hub { 0.05 } else { 0.034 };
        scene.add_part(
            &ellipse([0.0, 0.0], r, r, 14),
            *p,
            X,
            Y,
            if hub { 1.0 } else { 0.0 },
            0.05,
        );
    }
    scene.finish(0.72)
}

pub(crate) fn knot(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(camera(t));
    let count = (plan.shapes / 2).min(36);
    let path = |u: f64| {
        let p = [
            u.sin() + 2.0 * (2.0 * u).sin(),
            u.cos() - 2.0 * (2.0 * u).cos(),
            -(3.0 * u).sin(),
        ];
        let (sin, cos) = (t * 0.15).sin_cos();
        let p = [p[0] * cos - p[1] * sin, p[0] * sin + p[1] * cos, p[2]];
        tilt(spin(mul(p, 0.21), 0.3 * (t * 0.23).sin()), 0.2)
    };
    for k in 0..count {
        let u = TAU * k as f64 / count as f64 + t * 0.22;
        let centre = path(u);
        let heading = unit(sub(path(u + 0.01), path(u - 0.01)));
        let side = across(heading);
        let mix = if k % 6 == 0 { 1.0 } else { 0.0 };
        let (w, h) = (0.075, 0.048);
        let origin = sub(centre, add(mul(heading, w), mul(side, h)));
        scene.add_part(
            &square(w, h)
                .iter()
                .map(|[x, y]| [x + w, y + h])
                .collect::<Vec<_>>(),
            origin,
            heading,
            side,
            mix,
            0.0,
        );
        scene.add_part(
            &[[0.0, 2.0 * h], [2.0 * w, 2.0 * h], [w, h * 0.7]],
            origin,
            heading,
            side,
            mix,
            0.002,
        );
    }
    scene.finish(0.72)
}
