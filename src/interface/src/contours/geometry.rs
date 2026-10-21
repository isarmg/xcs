//! ZXS's G4 curvature profile and error-bounded SVG tessellation.
//! Analytic curvature bounds control each parallel-curve interpolation budget;
//! the emitted SVG chords themselves do not have G4 geometric continuity.
use super::profile as source;
pub use super::profile::{
    CIRCULAR_ARC_RATIO, INTEGRATION_INTERVALS, NORMALIZED_MAX_CURVATURE, PROFILE,
    TRANSITION_PARAMETER,
};
use data_serialization::Serialize;
use std::{cell::RefCell, collections::VecDeque, f64::consts::PI, rc::Rc};

#[derive(Clone, Copy, Debug, Serialize)]
#[serialization(rename_all = "camelCase")]
pub struct Vector {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serialization(rename_all = "camelCase")]
pub struct Profile {
    pub x: f64,
    pub y: f64,
    pub first: Vector,
    pub second: Vector,
    pub third: Vector,
    pub fourth: Vector,
    pub speed: f64,
    pub heading: f64,
    pub curvature: f64,
    pub curvature_derivative: f64,
    pub curvature_second_derivative: f64,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serialization(rename_all = "camelCase")]
pub struct Bounds {
    pub intervals: usize,
    pub minimum_speed: f64,
    pub maximum_curvature: f64,
    pub minimum_radius: f64,
    pub convex: bool,
    pub arithmetic_relative_margin: f64,
    pub coordinate_integration_error_bound: f64,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serialization(rename_all = "camelCase")]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub heading: f64,
    #[serialization(skip_serializing_if = "Option::is_none")]
    pub parameter: Option<f64>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serialization(rename_all = "camelCase")]
pub struct Source {
    pub extent: f64,
    pub radius: f64,
    pub offset: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serialization(rename_all = "camelCase")]
pub struct Curve {
    pub extent: f64,
    pub radius: f64,
    pub points: Vec<Point>,
    pub source: Source,
}

#[derive(Debug, Serialize)]
#[serialization(rename_all = "camelCase")]
pub struct Geometry {
    pub width: f64,
    pub height: f64,
    pub extent: f64,
    pub radius: f64,
    pub path: String,
    pub points: Vec<Point>,
    pub corners: Vec<Curve>,
    pub canonical: bool,
    pub error_bound: f64,
    #[serialization(skip)]
    pub tolerance: f64,
    #[serialization(skip)]
    offset_paths: RefCell<Cache<u64, String>>,
}

#[derive(Debug)]
struct Cache<K, V>(VecDeque<(K, V)>);

impl<K: PartialEq, V: Clone> Cache<K, V> {
    fn new() -> Self {
        Self(VecDeque::new())
    }

    fn get(&mut self, key: &K) -> Option<V> {
        let index = self.0.iter().position(|(item, _)| item == key)?;
        let entry = self.0.remove(index)?;
        let value = entry.1.clone();
        self.0.push_back(entry);
        Some(value)
    }

    fn set(&mut self, key: K, value: V, limit: usize) -> V {
        if let Some(index) = self.0.iter().position(|(item, _)| item == &key) {
            self.0.remove(index);
        }
        self.0.push_back((key, value.clone()));
        if self.0.len() > limit {
            self.0.pop_front();
        }
        value
    }
}

#[derive(Clone)]
struct Sample {
    points: Vec<Point>,
    bound: f64,
}

thread_local! {
    static BOUNDS: Bounds = analytic_bounds();
    static GEOMETRIES: RefCell<Cache<[u64; 4], Rc<Geometry>>> = RefCell::new(Cache::new());
    static SAMPLES: RefCell<Cache<[u64; 2], Sample>> = RefCell::new(Cache::new());
}

fn key(value: f64) -> u64 {
    // JavaScript Map keys and its original string keys identify -0 with +0.
    if value == 0.0 { 0 } else { value.to_bits() }
}

fn finite(value: f64, name: &str) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{name} must be finite"))
    }
}

/// Analytic position and Frenet derivatives for normalized total arc length.
pub fn profile(t: f64) -> Result<Profile, String> {
    finite(t, "t")?;
    if !(0.0..=1.0).contains(&t) {
        return Err("t must be in [0, 1]".into());
    }
    let q = source::evaluate(t);
    let (sin, cos) = q.heading.sin_cos();
    let k = q.curvature;
    let ks = q.curvature_derivative;
    let kss = q.curvature_second_derivative;
    let speed = source::SPEED;
    // T=(cos θ,sin θ), N=(-sin θ,cos θ), d/dt = speed*d/ds.
    Ok(Profile {
        x: q.x,
        y: q.y,
        first: Vector {
            x: speed * cos,
            y: speed * sin,
        },
        second: Vector {
            x: -speed.powi(2) * k * sin,
            y: speed.powi(2) * k * cos,
        },
        third: Vector {
            x: speed.powi(3) * (-ks * sin - k.powi(2) * cos),
            y: speed.powi(3) * (ks * cos - k.powi(2) * sin),
        },
        fourth: Vector {
            x: speed.powi(4) * (-(kss - k.powi(3)) * sin - 3.0 * k * ks * cos),
            y: speed.powi(4) * ((kss - k.powi(3)) * cos - 3.0 * k * ks * sin),
        },
        speed,
        heading: q.heading,
        curvature: k,
        curvature_derivative: ks,
        curvature_second_derivative: kss,
    })
}

/// q(u)=6u⁵-15u⁴+10u³ has q'(u)=30u²(1-u)²>=0, so 0<=κ<=K.
/// Arc-length speed is constant, and the central circle attains κ=K. The
/// relative margin reserves room for floating-point arithmetic; coordinates
/// also carry the separately derived Simpson truncation bound.
fn analytic_bounds() -> Bounds {
    let margin = 1e-10;
    let maximum_curvature = NORMALIZED_MAX_CURVATURE * (1.0 + margin);
    Bounds {
        intervals: INTEGRATION_INTERVALS,
        minimum_speed: source::SPEED * (1.0 - margin),
        maximum_curvature,
        minimum_radius: 1.0 / maximum_curvature,
        convex: true,
        arithmetic_relative_margin: margin,
        coordinate_integration_error_bound: source::integration_error_bound(),
    }
}

pub fn bounds() -> Bounds {
    BOUNDS.with(|value| *value)
}

/// Match `Number(value.toFixed(6)).toString()` without a JavaScript runtime.
/// Integer arithmetic rounds the exact binary input, including decimal ties;
/// multiplying by 1e6 first or Rust's ties-to-even formatting would differ.
pub fn fmt(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-Infinity"
        } else {
            "Infinity"
        }
        .into();
    }
    let absolute = value.abs();
    if absolute >= 1e21 {
        let scientific = format!("{value:e}");
        let (mantissa, exponent) = scientific.split_once('e').expect("scientific notation");
        let exponent: i32 = exponent.parse().expect("decimal exponent");
        return format!("{mantissa}e{exponent:+}");
    }
    let bits = absolute.to_bits();
    let encoded_exponent = ((bits >> 52) & 0x7ff) as i32;
    let (mantissa, exponent) = if encoded_exponent == 0 {
        (bits & ((1_u64 << 52) - 1), -1074)
    } else {
        (
            (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52),
            encoded_exponent - 1023 - 52,
        )
    };
    let numerator = u128::from(mantissa) * 1_000_000;
    let rounded = if exponent >= 0 {
        numerator << exponent as u32
    } else {
        let shift = (-exponent) as u32;
        if shift >= 128 {
            0
        } else {
            let quotient = numerator >> shift;
            let remainder = numerator - (quotient << shift);
            quotient + u128::from(remainder >= (1_u128 << (shift - 1)))
        }
    };
    if rounded == 0 {
        return "0".into();
    }
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let integer = rounded / 1_000_000;
    let fraction = rounded % 1_000_000;
    let decimal = if fraction == 0 {
        format!("{sign}{integer}")
    } else {
        let fraction = format!("{fraction:06}");
        format!("{sign}{integer}.{}", fraction.trim_end_matches('0'))
    };
    // toFixed returns a decimal string, but the original Number conversion
    // then prints the shortest round-tripping number (relevant above 2^53).
    decimal.parse::<f64>().expect("rounded decimal").to_string()
}

fn path(points: &[Point], offset: f64) -> String {
    let mut result = String::new();
    for (index, q) in points.iter().enumerate() {
        result.push(if index == 0 { 'M' } else { 'L' });
        result.push_str(&fmt(q.x - offset));
        result.push(' ');
        result.push_str(&fmt(q.y - offset));
    }
    result.push('Z');
    result
}

fn profile_point(t: f64, offset: f64) -> Point {
    let q = profile(t).expect("sampling uses a unit-interval parameter");
    Point {
        x: q.x - offset * q.heading.sin(),
        y: q.y + offset * q.heading.cos() - offset,
        heading: q.heading,
        parameter: Some(t),
    }
}

struct Sampler {
    offset: f64,
    budget: f64,
    points: Vec<Point>,
    bound: f64,
}

impl Sampler {
    fn visit(&mut self, interval: [f64; 2], depth: usize) -> Result<(), String> {
        let [start, end] = interval;
        // For γ+dN, d²/dt² = speed²[(1-dκ)κN-dκ_sT]. Its norm
        // is bounded globally even where a segment crosses a circle junction.
        let second = source::parallel_second_derivative_bound(self.offset);
        let error = second * (end - start).powi(2) / 8.0 * (1.0 + 1e-10);
        if error.is_finite() && error <= self.budget {
            self.bound = self.bound.max(error);
            self.points.push(profile_point(end, self.offset));
            return Ok(());
        }
        if depth == 24 {
            return Err("Corner sampling cannot meet the requested budget".into());
        }
        let middle = (start + end) / 2.0;
        self.visit([start, middle], depth + 1)?;
        self.visit([middle, end], depth + 1)
    }
}

fn sample_profile(offset: f64, budget: f64) -> Result<Sample, String> {
    let key = [key(offset), key(budget)];
    if let Some(cached) = SAMPLES.with(|cache| cache.borrow_mut().get(&key)) {
        return Ok(cached);
    }
    let mut sampler = Sampler {
        offset,
        budget,
        points: vec![profile_point(0.0, offset)],
        bound: 0.0,
    };
    sampler.visit([0.0, 1.0], 0)?;
    let sampled = Sample {
        points: sampler.points,
        bound: sampler.bound,
    };
    Ok(SAMPLES.with(|cache| cache.borrow_mut().set(key, sampled, 96)))
}

fn source_for(extent: f64) -> Source {
    Source {
        extent,
        radius: extent * bounds().minimum_radius,
        offset: 0.0,
    }
}

fn regular(source: Source) -> bool {
    source.extent > 0.0
        && source.offset
            < source.radius - (source.extent * f64::EPSILON * 16.0).max(source.radius * 1e-10)
}

struct LocalCurve {
    extent: f64,
    radius: f64,
    points: Vec<Point>,
    bound: f64,
}

fn local_curve(source: Source, tolerance: f64) -> Result<LocalCurve, String> {
    if source.extent == 0.0 {
        return Ok(LocalCurve {
            extent: 0.0,
            radius: 0.0,
            points: vec![
                Point {
                    x: 0.0,
                    y: 0.0,
                    heading: 0.0,
                    parameter: None,
                },
                Point {
                    x: 0.0,
                    y: 0.0,
                    heading: PI / 2.0,
                    parameter: None,
                },
            ],
            bound: 0.0,
        });
    }
    if !regular(source) {
        return Err("Corner offset is not certified regular".into());
    }
    let size = source.extent;
    let allowance = 2e-6 + size * 1e-10;
    if allowance >= tolerance / 2.0 {
        return Err("Tolerance is smaller than arithmetic/serialization allowance".into());
    }
    let sampled = sample_profile(source.offset / size, tolerance / (2.0 * size))?;
    Ok(LocalCurve {
        extent: source.extent - source.offset,
        radius: source.radius - source.offset,
        points: sampled
            .points
            .iter()
            .map(|q| Point {
                x: q.x * size,
                y: q.y * size,
                ..*q
            })
            .collect(),
        bound: sampled.bound * size + allowance,
    })
}

fn transform(q: Point, index: usize, width: f64, height: f64, extent: f64) -> Point {
    match index {
        0 => Point {
            x: width - extent + q.x,
            ..q
        },
        1 => Point {
            x: width - q.y,
            y: height - extent + q.x,
            heading: q.heading + PI / 2.0,
            ..q
        },
        2 => Point {
            x: extent - q.x,
            y: height - q.y,
            heading: q.heading + PI,
            ..q
        },
        _ => Point {
            x: q.y,
            y: extent - q.x,
            heading: q.heading + 3.0 * PI / 2.0,
            ..q
        },
    }
}

fn assemble(
    width: f64,
    height: f64,
    source: Source,
    tolerance: f64,
    canonical: bool,
) -> Result<Geometry, String> {
    let local = local_curve(source, tolerance)?;
    let error_bound = local.bound + (2e-6 + width.max(height) * f64::EPSILON * 16.0);
    if error_bound > tolerance {
        return Err("Corner cannot meet the requested coordinate budget".into());
    }
    let corners: Vec<_> = (0..4)
        .map(|index| Curve {
            extent: local.extent,
            radius: local.radius,
            source,
            points: local
                .points
                .iter()
                .map(|q| transform(*q, index, width, height, local.extent))
                .collect(),
        })
        .collect();
    let mut points = vec![Point {
        x: corners[3].extent,
        y: 0.0,
        heading: 0.0,
        parameter: None,
    }];
    points.extend(
        corners
            .iter()
            .flat_map(|corner| corner.points.iter().copied()),
    );
    Ok(Geometry {
        width,
        height,
        extent: local.extent,
        radius: local.radius,
        path: path(&points, 0.0),
        points,
        corners,
        canonical,
        error_bound,
        tolerance,
        offset_paths: RefCell::new(Cache::new()),
    })
}

/// Clamp identical physical corner boxes to half the short side, retaining
/// the original profile without stretching one axis independently.
pub fn create(
    width: f64,
    height: f64,
    extent: f64,
    tolerance: f64,
) -> Result<Rc<Geometry>, String> {
    finite(width, "width")?;
    finite(height, "height")?;
    finite(extent, "extent")?;
    finite(tolerance, "tolerance")?;
    if width <= 0.0 || height <= 0.0 || extent < 0.0 || tolerance <= 0.0 {
        return Err("Invalid corner dimensions or tolerance".into());
    }
    let extent = extent.min(width / 2.0).min(height / 2.0);
    let key = [key(width), key(height), key(extent), key(tolerance)];
    if let Some(cached) = GEOMETRIES.with(|cache| cache.borrow_mut().get(&key)) {
        return Ok(cached);
    }
    let result = Rc::new(assemble(
        width,
        height,
        source_for(extent),
        tolerance,
        true,
    )?);
    Ok(GEOMETRIES.with(|cache| cache.borrow_mut().set(key, result, 96)))
}

/// Resample the analytic parallel profile with its own derivative bound;
/// translating previously sampled coarse chords would lose the error budget.
pub fn offset_path(geometry: &Geometry, offset: f64) -> Result<String, String> {
    finite(offset, "offset")?;
    if offset == 0.0 {
        return Ok(geometry.path.clone());
    }
    if geometry.width + offset * 2.0 <= 0.0 || geometry.height + offset * 2.0 <= 0.0 {
        return Err("Inset leaves no positive shadow dimensions".into());
    }
    if offset < 0.0 && geometry.radius > 0.0 && offset <= -geometry.radius {
        return Err("Inset exceeds certified minimum curvature radius".into());
    }
    if let Some(cached) = geometry.offset_paths.borrow_mut().get(&key(offset)) {
        return Ok(cached);
    }
    let source = geometry.corners[0].source;
    let source = if source.extent == 0.0 {
        source_for(0.0)
    } else {
        Source {
            offset: source.offset - offset,
            ..source
        }
    };
    let expanded = assemble(
        geometry.width + offset * 2.0,
        geometry.height + offset * 2.0,
        source,
        geometry.tolerance,
        false,
    )?;
    let result = path(&expanded.points, offset);
    Ok(geometry
        .offset_paths
        .borrow_mut()
        .set(key(offset), result, 16))
}

#[cfg(test)]
#[path = "../../../../test/interface/unit/contours/geometry/tests.rs"]
mod tests;
