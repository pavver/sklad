//! The curves of `docs/foundations/motion.md`.

/// `cubic-bezier(x1, y1, x2, y2)` at progress `t`, as CSS computes it.
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let curve = |a: f32, b: f32, s: f32| {
        let inverse = 1.0 - s;
        3.0 * inverse * inverse * s * a + 3.0 * inverse * s * s * b + s * s * s
    };
    let slope = |a: f32, b: f32, s: f32| {
        let inverse = 1.0 - s;
        3.0 * inverse * inverse * a + 6.0 * inverse * s * (b - a) + 3.0 * s * s * (1.0 - b)
    };
    let mut s = t;
    for _ in 0..8 {
        let error = curve(x1, x2, s) - t;
        let gradient = slope(x1, x2, s);
        if error.abs() < 1e-5 || gradient.abs() < 1e-6 {
            break;
        }
        s = (s - error / gradient).clamp(0.0, 1.0);
    }
    curve(y1, y2, s)
}

/// Standard: ordinary state colours and value jumps.
pub fn standard(t: f32) -> f32 {
    cubic_bezier(0.4, 0.0, 0.2, 1.0, t)
}

/// Smooth: the exit fade of a result.
pub fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_curves_start_at_zero_and_end_at_one() {
        for curve in [standard, smooth] {
            assert!(curve(0.0).abs() < 1e-3);
            assert!((curve(1.0) - 1.0).abs() < 1e-3);
        }
    }

    #[test]
    fn the_standard_curve_is_ahead_of_linear_in_its_middle() {
        assert!(standard(0.5) > 0.7);
    }

    #[test]
    fn a_curve_with_overshoot_passes_its_end() {
        let peak = (1..100)
            .map(|step| cubic_bezier(0.4, -0.2, 0.4, 1.2, step as f32 / 100.0))
            .fold(0.0_f32, f32::max);
        assert!(peak > 1.0);
    }
}
