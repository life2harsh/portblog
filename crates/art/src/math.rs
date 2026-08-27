#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[inline]
pub fn smootherstep(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
pub fn hash(mut n: u64) -> f64 {
    n = n
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    n ^= n >> 33;
    n = n.wrapping_mul(0xff51afd7ed558ccd);
    n ^= n >> 33;
    n = n.wrapping_mul(0xc4ceb9fe1a85ec53);
    n ^= n >> 33;
    (n as f64) / (u64::MAX as f64)
}

pub fn value_noise_2d(x: f64, y: f64, seed: u64) -> f64 {
    let xi = x.floor() as i64;
    let yi = y.floor() as i64;
    let xf = x - xi as f64;
    let yf = y - yi as f64;

    let u = smootherstep(xf);
    let v = smootherstep(yf);

    let h = |ix: i64, iy: i64| -> f64 {
        let k = (ix.wrapping_mul(374761393) ^ iy.wrapping_mul(668265263)) as u64;
        hash(k.wrapping_add(seed)) * 2.0 - 1.0
    };

    let a = h(xi, yi);
    let b = h(xi + 1, yi);
    let c = h(xi, yi + 1);
    let d = h(xi + 1, yi + 1);

    lerp(lerp(a, b, u), lerp(c, d, u), v)
}

pub fn fbm(x: f64, y: f64, octaves: u8, seed: u64) -> f64 {
    let mut value = 0.0f64;
    let mut amplitude = 0.5f64;
    let mut frequency = 1.0f64;
    let mut max_value = 0.0f64;

    for i in 0..octaves {
        value += value_noise_2d(
            x * frequency,
            y * frequency,
            seed.wrapping_add(i as u64 * 997),
        ) * amplitude;
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value / max_value
}
