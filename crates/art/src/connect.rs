use std::f64::consts::{PI, TAU};

use crate::field::{Camera, Plan, Shape, V3, X, Y, Z, add, cross, ellipse, mul, square, sub, unit};
use crate::math::smootherstep;

fn spin(p: V3, angle: f64) -> V3 {
    let (sin, cos) = angle.sin_cos();
    [p[0] * cos + p[2] * sin, p[1], p[2] * cos - p[0] * sin]
}

fn tilt(p: V3, angle: f64) -> V3 {
    let (sin, cos) = angle.sin_cos();
    [p[0], p[1] * cos - p[2] * sin, p[1] * sin + p[2] * cos]
}

fn turn(v: V3, axis: V3, angle: f64) -> V3 {
    let (sin, cos) = angle.sin_cos();
    let along = v[0] * axis[0] + v[1] * axis[1] + v[2] * axis[2];
    add(
        add(mul(v, cos), mul(cross(axis, v), sin)),
        mul(axis, along * (1.0 - cos)),
    )
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

pub(crate) fn fork(plan: &Plan) -> Shape {
    let t = plan.t;
    let mut scene = plan.scene(camera(t));
    let depth = if plan.shapes >= 64 { 6 } else { 5 };
    let cycle = (t / 14.0).fract() * 14.0;
    let wither = 1.0 - smootherstep((cycle - 13.0) / 0.9);
    let turning = t * 0.3;
    let mut stack: Vec<(V3, V3, V3, f64, usize)> = vec![([0.0, -0.98, 0.0], Y, X, 0.5, 0)];
    while let Some((start, direction, side, length, level)) = stack.pop() {
        let grow = smootherstep((cycle - 0.1 - level as f64 * 0.22) / 0.45) * wither;
        if grow < 0.01 {
            continue;
        }
        let reach = length * grow;
        let end = add(start, mul(direction, reach));
        let width = 0.042 * 0.68f64.powi(level as i32);
        let (a, b) = (spin(start, turning), spin(end, turning));
        let axis = unit(sub(b, a));
        let span = sub(b, a);
        let span = (span[0] * span[0] + span[1] * span[1] + span[2] * span[2]).sqrt();
        scene.add_free(
            &[
                [0.0, -width],
                [span, -width * 0.7],
                [span, width * 0.7],
                [0.0, width],
            ],
            a,
            axis,
            across(axis),
            if level + 1 == depth { 1.0 } else { 0.0 },
        );
        if level + 1 < depth {
            let spread = 0.5 + 0.1 * (t * 1.1 + level as f64 * 0.9).sin();
            let wind = 0.06 * (t * 1.7 - level as f64 * 0.6).sin();
            let next_side = turn(side, direction, PI * 0.5);
            for sign in [-1.0, 1.0] {
                let heading = unit(turn(direction, side, sign * spread + wind));
                stack.push((end, heading, next_side, length * 0.78, level + 1));
            }
        }
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
