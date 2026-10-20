//! Arc-length curvature construction from “给矩形倒圆角 (2)”, G4 section.
//!
//! This is a transition / circle / reflected-transition curve, not a Bézier
//! fit or an Apple API. The retained circular-angle ratio p = 0.4 is ZXS's
//! selected appearance; the article does not specify an Apple default.

use std::f64::consts::PI;

pub const PROFILE: &str = "zxs-g4";
pub const CIRCULAR_ARC_RATIO: f64 = 0.4;
/// K = A ∫₀¹(cos θ(u) + sin θ(u)) du + cos(A/2) - sin(A/2).
/// Thus a unit corner extent has circle radius 1/K.
pub const NORMALIZED_MAX_CURVATURE: f64 = 1.486_024_529_088_757;
pub const INTEGRATION_INTERVALS: usize = 512;
pub const TRANSITION_PARAMETER: f64 = 0.375;

const A: f64 = PI * (1.0 - CIRCULAR_ARC_RATIO) / 2.0;
const RADIUS: f64 = 1.0 / NORMALIZED_MAX_CURVATURE;
const TRANSITION_LENGTH: f64 = A * RADIUS;
pub(super) const SPEED: f64 = (2.0 * A + PI * CIRCULAR_ARC_RATIO / 2.0) * RADIUS;

#[derive(Clone, Copy, Default)]
struct Integral {
    x: f64,
    y: f64,
}

pub(super) struct Sample {
    pub x: f64,
    pub y: f64,
    pub heading: f64,
    pub curvature: f64,
    pub curvature_derivative: f64,
    pub curvature_second_derivative: f64,
}

/// Integral of q(u) = 6u⁵ - 15u⁴ + 10u³, multiplied by A = L/R.
fn transition_heading(u: f64) -> f64 {
    A * u.powi(4) * (u * (u - 3.0) + 2.5)
}

fn tangent(u: f64) -> Integral {
    let (sin, cos) = transition_heading(u).sin_cos();
    Integral { x: cos, y: sin }
}

fn simpson(start: f64, end: f64) -> Integral {
    let first = tangent(start);
    let middle = tangent((start + end) / 2.0);
    let last = tangent(end);
    let scale = (end - start) / 6.0;
    Integral {
        x: scale * (first.x + 4.0 * middle.x + last.x),
        y: scale * (first.y + 4.0 * middle.y + last.y),
    }
}

thread_local! {
    static INTEGRALS: [Integral; INTEGRATION_INTERVALS + 1] = {
        let mut table = [Integral::default(); INTEGRATION_INTERVALS + 1];
        for index in 1..=INTEGRATION_INTERVALS {
            let step = simpson(
                (index - 1) as f64 / INTEGRATION_INTERVALS as f64,
                index as f64 / INTEGRATION_INTERVALS as f64,
            );
            table[index] = Integral {
                x: table[index - 1].x + step.x,
                y: table[index - 1].y + step.y,
            };
        }
        table
    };
}

fn integral(u: f64) -> Integral {
    INTEGRALS.with(|table| {
        let index =
            ((u * INTEGRATION_INTERVALS as f64).floor() as usize).min(INTEGRATION_INTERVALS);
        let remainder = simpson(index as f64 / INTEGRATION_INTERVALS as f64, u);
        Integral {
            x: table[index].x + remainder.x,
            y: table[index].y + remainder.y,
        }
    })
}

/// Vector Simpson truncation bound for the normalized transition coordinates.
/// For z(u)=exp(iθ(u)), |z''''| <= |θ''''| + 4|θ'θ'''| + 3|θ''|²
/// + 6|θ'|²|θ''| + |θ'|⁴. Here |q|<=1, |q'|<=15/8,
/// |q''|<=10/√3 and |q'''|<=60. A Simpson cell of length h contributes
/// at most M h⁵/2880. Include all 512 cells plus a residual cell. This
/// truncation bound is separate from the geometry's f64/serialization margin.
pub(super) fn integration_error_bound() -> f64 {
    let fourth_bound = A * 60.0
        + 4.0 * A.powi(2) * (10.0 / 3.0_f64.sqrt())
        + 3.0 * (A * 1.875).powi(2)
        + 6.0 * A.powi(3) * 1.875
        + A.powi(4);
    let n = INTEGRATION_INTERVALS as f64;
    TRANSITION_LENGTH * fourth_bound * (1.0 + 1.0 / n) / (2880.0 * n.powi(4))
}

pub(super) fn maximum_curvature_derivative() -> f64 {
    NORMALIZED_MAX_CURVATURE.powi(2) / A * 1.875
}

/// A bound on d²/dt² of the inward parallel curve γ(t)+dN(t), with
/// t = normalized total arc length. This also applies to negative offsets.
pub(super) fn parallel_second_derivative_bound(offset: f64) -> f64 {
    let k = NORMALIZED_MAX_CURVATURE;
    SPEED.powi(2) * (k * (1.0 + offset.abs() * k) + offset.abs() * maximum_curvature_derivative())
}

fn transition(t: f64) -> Sample {
    let u = (t / TRANSITION_PARAMETER).clamp(0.0, 1.0);
    // Evaluate q with symmetry to avoid cancellation near u = 1.
    let q = |v: f64| v.powi(3) * (10.0 + v * (-15.0 + 6.0 * v));
    let curvature = NORMALIZED_MAX_CURVATURE * if u <= 0.5 { q(u) } else { 1.0 - q(1.0 - u) };
    let first = 30.0 * u.powi(2) * (1.0 - u).powi(2);
    let second = 60.0 * u * (1.0 - u) * (1.0 - 2.0 * u);
    let point = integral(u);
    Sample {
        x: point.x * TRANSITION_LENGTH,
        y: point.y * TRANSITION_LENGTH,
        heading: transition_heading(u),
        curvature,
        curvature_derivative: NORMALIZED_MAX_CURVATURE / TRANSITION_LENGTH * first,
        curvature_second_derivative: NORMALIZED_MAX_CURVATURE / TRANSITION_LENGTH.powi(2) * second,
    }
}

pub(super) fn evaluate(t: f64) -> Sample {
    if t <= TRANSITION_PARAMETER {
        return transition(t);
    }
    if t >= 1.0 - TRANSITION_PARAMETER {
        let first = transition(1.0 - t);
        return Sample {
            x: 1.0 - first.y,
            y: 1.0 - first.x,
            heading: PI / 2.0 - first.heading,
            curvature_derivative: -first.curvature_derivative,
            ..first
        };
    }
    let first = transition(TRANSITION_PARAMETER);
    let heading = A / 2.0 + SPEED * NORMALIZED_MAX_CURVATURE * (t - TRANSITION_PARAMETER);
    Sample {
        x: first.x + RADIUS * (heading.sin() - (A / 2.0).sin()),
        y: first.y + RADIUS * ((A / 2.0).cos() - heading.cos()),
        heading,
        curvature: NORMALIZED_MAX_CURVATURE,
        curvature_derivative: 0.0,
        curvature_second_derivative: 0.0,
    }
}
