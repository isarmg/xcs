//! ZXS's G4 curvature profile and error-bounded SVG tessellation.
//! Analytic curvature bounds control each parallel-curve interpolation budget;
//! the emitted SVG chords themselves do not have G4 geometric continuity.
use super::profile as source;
pub use super::profile::{
    CIRCULAR_ARC_RATIO, INTEGRATION_INTERVALS, NORMALIZED_MAX_CURVATURE, PROFILE,
    TRANSITION_PARAMETER,
};
use serde::Serialize;
use std::{cell::RefCell, collections::VecDeque, f64::consts::PI, rc::Rc};

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Vector {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub heading: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter: Option<f64>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub extent: f64,
    pub radius: f64,
    pub offset: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Curve {
    pub extent: f64,
    pub radius: f64,
    pub points: Vec<Point>,
    pub source: Source,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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
    #[serde(skip)]
    pub tolerance: f64,
    #[serde(skip)]
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
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{actual} != {expected}"
        );
    }

    // Independent composite Simpson reference over total normalized arc
    // length. It integrates the tangent directly, never production coordinates,
    // transition lookup tables, or the production circle-position formula.
    const REFERENCE_INTERVALS: usize = 16_384;
    const REFERENCE_K: f64 = 1.486_024_529_088_757;

    fn reference_heading(t: f64) -> f64 {
        if t > 0.625 {
            return PI / 2.0 - reference_heading(1.0 - t);
        }
        if t > 0.375 {
            return PI * (0.15 + 0.8 * (t - 0.375));
        }
        let u = t / 0.375;
        PI * 0.3 * (u.powi(6) - 3.0 * u.powi(5) + 2.5 * u.powi(4))
    }

    fn reference_step(start: f64, end: f64) -> Vector {
        let mut point = Vector { x: 0.0, y: 0.0 };
        for (t, weight) in [(start, 1.0), ((start + end) / 2.0, 4.0), (end, 1.0)] {
            let angle = reference_heading(t);
            point.x += weight * angle.cos();
            point.y += weight * angle.sin();
        }
        let scale = (end - start) * (PI * 0.8 / REFERENCE_K) / 6.0;
        Vector {
            x: point.x * scale,
            y: point.y * scale,
        }
    }

    thread_local! {
        static REFERENCE_POINTS: Vec<Vector> = {
            let mut table = vec![Vector { x: 0.0, y: 0.0 }];
            for index in 1..=REFERENCE_INTERVALS {
                let step = reference_step(
                    (index - 1) as f64 / REFERENCE_INTERVALS as f64,
                    index as f64 / REFERENCE_INTERVALS as f64,
                );
                let previous = table[index - 1];
                table.push(Vector { x: previous.x + step.x, y: previous.y + step.y });
            }
            table
        };
    }

    fn reference(t: f64, source: Source) -> Vector {
        let q = REFERENCE_POINTS.with(|table| {
            let index =
                ((t * REFERENCE_INTERVALS as f64).floor() as usize).min(REFERENCE_INTERVALS);
            let step = reference_step(index as f64 / REFERENCE_INTERVALS as f64, t);
            Vector {
                x: table[index].x + step.x,
                y: table[index].y + step.y,
            }
        });
        let heading = reference_heading(t);
        Vector {
            x: q.x * source.extent - source.offset * heading.sin(),
            y: q.y * source.extent + source.offset * (heading.cos() - 1.0),
        }
    }

    fn local(q: Point, corner: &Curve, geometry: &Geometry, index: usize) -> Vector {
        match index {
            0 => Vector {
                x: q.x - (geometry.width - corner.extent),
                y: q.y,
            },
            1 => Vector {
                x: q.y - (geometry.height - corner.extent),
                y: geometry.width - q.x,
            },
            2 => Vector {
                x: corner.extent - q.x,
                y: geometry.height - q.y,
            },
            _ => Vector {
                x: corner.extent - q.y,
                y: q.x,
            },
        }
    }

    fn assert_convex(geometry: &Geometry) {
        let mut distinct: Vec<Point> = Vec::new();
        for point in &geometry.points {
            if distinct
                .last()
                .is_none_or(|last| (point.x - last.x).hypot(point.y - last.y) > 1e-8)
            {
                distinct.push(*point);
            }
        }
        if (distinct[0].x - distinct[distinct.len() - 1].x)
            .hypot(distinct[0].y - distinct[distinct.len() - 1].y)
            < 1e-8
        {
            distinct.pop();
        }
        for index in 0..distinct.len() {
            let a = distinct[index];
            let b = distinct[(index + 1) % distinct.len()];
            let c = distinct[(index + 2) % distinct.len()];
            assert!((b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x) >= -1e-7);
        }
        for q in &geometry.points {
            assert!(q.x >= -1e-8 && q.x <= geometry.width + 1e-8);
            assert!(q.y >= -1e-8 && q.y <= geometry.height + 1e-8);
        }
    }

    fn check_error(geometry: &Geometry) {
        let mut observed: f64 = 0.0;
        for (index, corner) in geometry.corners.iter().enumerate() {
            if corner.extent == 0.0 {
                continue;
            }
            for pair in corner.points.windows(2) {
                let [a, b] = [pair[0], pair[1]];
                let rounded = |p: Point| Point {
                    x: fmt(p.x).parse().unwrap(),
                    y: fmt(p.y).parse().unwrap(),
                    ..p
                };
                let al = local(rounded(a), corner, geometry, index);
                let bl = local(rounded(b), corner, geometry, index);
                for j in 1..16 {
                    let fraction = f64::from(j) / 16.0;
                    let t = a.parameter.unwrap()
                        + (b.parameter.unwrap() - a.parameter.unwrap()) * fraction;
                    let q = reference(t, corner.source);
                    observed = observed.max(
                        (q.x - al.x * (1.0 - fraction) - bl.x * fraction)
                            .hypot(q.y - al.y * (1.0 - fraction) - bl.y * fraction),
                    );
                }
            }
        }
        assert!(
            observed <= geometry.error_bound,
            "observed {observed} > {}",
            geometry.error_bound
        );
    }

    #[test]
    fn source_curvature_formula_normalizes_the_requested_corner_extent() {
        assert_eq!(PROFILE, "zxs-g4");
        assert_eq!(CIRCULAR_ARC_RATIO, 0.4);
        assert_eq!(TRANSITION_PARAMETER, 0.375);
        let diagonal = profile(0.5).unwrap();
        close(diagonal.x + diagonal.y, 1.0, 1e-10);
        close(diagonal.heading, PI / 4.0, 1e-12);
        close(diagonal.curvature, REFERENCE_K, 1e-12);
        let unit = Source {
            extent: 1.0,
            radius: 1.0 / REFERENCE_K,
            offset: 0.0,
        };
        for i in 0..=1000 {
            let t = f64::from(i) / 1000.0;
            let q = profile(t).unwrap();
            let independent = reference(t, unit);
            close(q.x, independent.x, 1e-10);
            close(q.y, independent.y, 1e-10);
        }
        let row = create(240.0, 36.0, 12.0, 0.02).unwrap();
        close(row.extent, 12.0, 1e-12);
        close(row.height - 2.0 * row.extent, 12.0, 1e-12);
        let list = create(240.0, 216.0, 12.0, 0.02).unwrap();
        close(list.extent, row.extent, 1e-12);
        let input = create(240.0, 44.0, 44.0 / 3.0, 0.02).unwrap();
        close(input.height - 2.0 * input.extent, input.extent, 1e-12);
    }

    #[test]
    fn analytic_endpoints_and_circle_junctions_are_g4() {
        let start = profile(0.0).unwrap();
        let end = profile(1.0).unwrap();
        close(start.x, 0.0, 1e-12);
        close(start.y, 0.0, 1e-12);
        close(end.x, 1.0, 1e-12);
        close(end.y, 1.0, 1e-12);
        close(start.heading, 0.0, 1e-12);
        close(end.heading, PI / 2.0, 1e-12);
        for q in [&start, &end] {
            close(q.curvature, 0.0, 1e-12);
            close(q.curvature_derivative, 0.0, 1e-12);
            close(q.curvature_second_derivative, 0.0, 1e-12);
            close(q.second.x.hypot(q.second.y), 0.0, 1e-12);
            close(q.third.x.hypot(q.third.y), 0.0, 1e-12);
            close(q.fourth.x.hypot(q.fourth.y), 0.0, 1e-12);
            assert!(q.speed > 0.0);
        }
        for t in [0.375, 0.625] {
            let q = profile(t).unwrap();
            close(q.curvature, REFERENCE_K, 1e-12);
            close(q.curvature_derivative, 0.0, 1e-12);
            close(q.curvature_second_derivative, 0.0, 1e-12);
            // Independent one-sided checks across both analytic pieces.
            let before = profile(t - 1e-9).unwrap();
            let after = profile(t + 1e-9).unwrap();
            close(before.x, after.x, 4e-9);
            close(before.y, after.y, 4e-9);
            close(before.curvature, after.curvature, 1e-10);
            close(
                before.curvature_derivative,
                after.curvature_derivative,
                1e-10,
            );
            close(
                before.curvature_second_derivative,
                after.curvature_second_derivative,
                1e-5,
            );
            close(before.fourth.x, after.fourth.x, 1e-4);
            close(before.fourth.y, after.fourth.y, 1e-4);
        }
    }

    #[test]
    fn analytic_bounds_cover_speed_curvature_offsets_and_integration() {
        let bound = bounds();
        assert_eq!(bound.intervals, INTEGRATION_INTERVALS);
        assert!(bound.convex);
        assert!(bound.minimum_radius > 0.672 && bound.minimum_radius < 0.674);
        assert!(bound.coordinate_integration_error_bound > 0.0);
        assert!(bound.coordinate_integration_error_bound < 1e-12);
        let mut maximum: f64 = 0.0;
        for i in 0..=5000 {
            let q = profile(f64::from(i) / 5000.0).unwrap();
            assert!(q.speed >= bound.minimum_speed);
            assert!(q.curvature >= 0.0 && q.curvature <= bound.maximum_curvature);
            maximum = maximum.max(q.curvature);
        }
        assert!(bound.maximum_curvature / maximum < 1.000_000_001);
        let a = PI * 0.3;
        let first = profile(0.375 / 2.0).unwrap();
        close(
            first.curvature_derivative,
            REFERENCE_K.powi(2) / a * 1.875,
            1e-12,
        );
        let last = profile(1.0 - 0.375 / 2.0).unwrap();
        close(
            last.curvature_derivative,
            -first.curvature_derivative,
            1e-12,
        );
    }

    #[test]
    fn frenet_derivatives_follow_the_integrated_curve_and_parallel_bounds() {
        let h = 1e-6;
        for t in [0.1, 0.25, 0.5, 0.75, 0.9] {
            let before = profile(t - h).unwrap();
            let q = profile(t).unwrap();
            let after = profile(t + h).unwrap();
            close((after.x - before.x) / (2.0 * h), q.first.x, 1e-8);
            close((after.y - before.y) / (2.0 * h), q.first.y, 1e-8);
            for (before, after, expected) in [
                (before.first, after.first, q.second),
                (before.second, after.second, q.third),
                (before.third, after.third, q.fourth),
            ] {
                close((after.x - before.x) / (2.0 * h), expected.x, 1e-6);
                close((after.y - before.y) / (2.0 * h), expected.y, 1e-6);
            }
        }
        for offset in [-4.0, -0.5, 0.0, 0.3, 0.6] {
            let bound = source::parallel_second_derivative_bound(offset);
            for index in 0..=1000 {
                let q = profile(f64::from(index) / 1000.0).unwrap();
                let actual = q.speed.powi(2)
                    * ((1.0 - offset * q.curvature) * q.curvature)
                        .hypot(offset * q.curvature_derivative);
                assert!(actual <= bound * (1.0 + 1e-12));
            }
        }
    }

    #[test]
    fn corner_boxes_are_convex_across_rectangles_and_small_extents() {
        for [width, height] in [
            [12.0, 48.0],
            [48.0, 12.0],
            [320.0, 216.0],
            [216.0, 144.0],
            [1024.0, 1024.0],
        ] {
            for extent in [0.0, 1e-12, 0.01, 27.0, 42.0, 1000.0] {
                let g = create(width, height, extent, 0.05).unwrap();
                assert_eq!(g.extent, extent.min(width / 2.0).min(height / 2.0));
                assert!(g.error_bound <= 0.05);
                assert_convex(&g);
                for corner in &g.corners {
                    close(corner.extent, g.extent, 1e-10);
                }
            }
        }
        let g = create(10.123456789, 20.987654321, 0.0, 0.05).unwrap();
        let rounding = (g.width - fmt(g.width).parse::<f64>().unwrap())
            .hypot(g.height - fmt(g.height).parse::<f64>().unwrap());
        assert!(rounding > 0.0 && rounding <= g.error_bound);
    }

    #[test]
    fn adaptive_error_budgets_bound_independently_evaluated_curves() {
        for [width, height, extent] in [
            [600.0, 400.0, 150.0],
            [536.0, 336.0, 118.0],
            [100.0, 80.0, 27.0],
            [72.0, 56.0, 56.0 / 3.0],
        ] {
            let g = create(width, height, extent, 0.02).unwrap();
            assert!(g.error_bound <= 0.02);
            assert!(g.corners.iter().all(|c| c.points.len() < 500));
            check_error(&g);
            assert_convex(&g);
        }
    }

    #[test]
    fn all_four_corners_preserve_identical_physical_coordinates() {
        for [width, height, extent] in [
            [240.0, 120.0, 27.0],
            [120.0, 240.0, 27.0],
            [127.0, 127.0, 127.0 / 3.0],
            [274.0, 144.0, 144.0 / 3.0],
            [48.0, 44.0, 27.0],
        ] {
            let g = create(width, height, extent, 0.02).unwrap();
            assert!(g.canonical);
            assert_eq!(g.extent, extent.min(width / 2.0).min(height / 2.0));
            for corner in &g.corners {
                assert_eq!(corner.extent, g.extent);
            }
            for i in 0..g.corners[0].points.len() {
                let first = local(g.corners[0].points[i], &g.corners[0], &g, 0);
                for index in 1..4 {
                    let q = local(g.corners[index].points[i], &g.corners[index], &g, index);
                    close(q.x, first.x, 1e-10);
                    close(q.y, first.y, 1e-10);
                }
            }
            check_error(&g);
            assert_convex(&g);
        }
    }

    #[test]
    fn parallel_offsets_resample_analytic_curves_and_reject_singularities() {
        let g = create(200.0, 120.0, 42.0, 0.05).unwrap();
        assert!(offset_path(&g, 3.0).unwrap().starts_with("M42 -3L158 -3"));
        assert_eq!(offset_path(&g, 0.0).unwrap(), g.path);
        assert!(offset_path(&g, -1.0).unwrap().starts_with("M42 1L158 1"));
        let square = create(200.0, 120.0, 0.0, 0.05).unwrap();
        assert!(
            offset_path(&square, -1.0)
                .unwrap()
                .starts_with("M1 1L199 1")
        );
        assert!(offset_path(&square, -60.0).is_err());
        assert!(offset_path(&g, -g.radius).is_err());
        // Check rounded emitted segments against independently evaluated offsets.
        let svg = offset_path(&g, 12.0).unwrap();
        let vertices: Vec<Vector> = svg
            .trim_end_matches('Z')
            .split(['M', 'L'])
            .filter(|part| !part.is_empty())
            .map(|part| {
                let (x, y) = part.split_once(' ').unwrap();
                Vector {
                    x: x.parse().unwrap(),
                    y: y.parse().unwrap(),
                }
            })
            .collect();
        let distance = |q: Vector, a: Vector, b: Vector| {
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let square = dx * dx + dy * dy;
            let t = if square == 0.0 {
                0.0
            } else {
                ((q.x - a.x) * dx + (q.y - a.y) * dy) / square
            }
            .clamp(0.0, 1.0);
            (q.x - a.x - t * dx).hypot(q.y - a.y - t * dy)
        };
        let mut observed: f64 = 0.0;
        for i in 0..=1000 {
            let mut q = reference(
                f64::from(i) / 1000.0,
                Source {
                    offset: -12.0,
                    ..g.corners[0].source
                },
            );
            q.x += 200.0 - 42.0;
            q.y -= 12.0;
            let closest = vertices
                .windows(2)
                .map(|p| distance(q, p[0], p[1]))
                .fold(f64::INFINITY, f64::min);
            observed = observed.max(closest);
        }
        assert!(observed <= 0.05, "offset error {observed}");
    }

    #[test]
    fn bounded_caches_reuse_geometry_and_discard_old_entries() {
        GEOMETRIES.with(|cache| cache.borrow_mut().0.clear());
        let g = create(200.0, 120.0, 42.0, 0.05).unwrap();
        assert!(Rc::ptr_eq(&g, &create(200.0, 120.0, 42.0, 0.05).unwrap()));
        for i in 0..110 {
            create(300.0 + f64::from(i), 150.0, 27.0, 0.05).unwrap();
        }
        assert!(!Rc::ptr_eq(&g, &create(200.0, 120.0, 42.0, 0.05).unwrap()));
        GEOMETRIES.with(|cache| assert_eq!(cache.borrow().0.len(), 96));
        for offset in 1..=20 {
            offset_path(&g, f64::from(offset)).unwrap();
        }
        assert_eq!(g.offset_paths.borrow().0.len(), 16);
    }

    #[test]
    fn invalid_coordinates_budgets_parameters_and_offsets_are_rejected() {
        for input in [
            [0.0, 120.0, 42.0, 0.05],
            [200.0, 120.0, -1.0, 0.05],
            [200.0, 120.0, 42.0, 0.0],
            [200.0, f64::INFINITY, 42.0, 0.05],
            [200.0, 120.0, 42.0, 1e-9],
            [1e16, 120.0, 0.0, 0.05],
        ] {
            assert!(create(input[0], input[1], input[2], input[3]).is_err());
        }
        for t in [-0.01, 1.01, f64::NAN, f64::INFINITY] {
            assert!(profile(t).is_err());
        }
        let g = create(200.0, 120.0, 42.0, 0.05).unwrap();
        for offset in [f64::NAN, f64::INFINITY, -f64::INFINITY] {
            assert!(offset_path(&g, offset).is_err());
        }
    }

    #[test]
    fn svg_decimal_format_preserves_javascript_rounding_and_negative_zero() {
        for (input, expected) in [
            (0.0, "0"),
            (-0.0, "0"),
            (0.0078125, "0.007813"),
            (-0.0078125, "-0.007813"),
            (1.0000015, "1.000001"),
            (0.00000049, "0"),
            (-0.00000049, "0"),
            (12.5, "12.5"),
            (1_000_000_000_000_000_128.0, "1000000000000000100"),
            (1e21, "1e+21"),
            (1e-300, "0"),
            (f64::INFINITY, "Infinity"),
            (-f64::INFINITY, "-Infinity"),
            (f64::NAN, "NaN"),
        ] {
            assert_eq!(fmt(input), expected, "{input}");
        }
    }

    #[test]
    fn serialized_public_shapes_match_browser_contracts() {
        let geometry = create(200.0, 120.0, 42.0, 0.05).unwrap();
        let json = serde_json::to_value(geometry.as_ref()).unwrap();
        assert!(json.get("errorBound").is_some());
        assert!(json.get("tolerance").is_none());
        assert!(json.get("offset_paths").is_none());
        assert!(json["points"][0].get("parameter").is_none());
        assert!(json["corners"][0]["points"][0].get("parameter").is_some());
        let profile = serde_json::to_value(profile(0.5).unwrap()).unwrap();
        assert!(profile.get("curvatureDerivative").is_some());
        assert!(profile.get("curvatureSecondDerivative").is_some());
        let bounds = serde_json::to_value(bounds()).unwrap();
        assert_eq!(bounds["intervals"], 512);
        assert!(bounds.get("minimumRadius").is_some());
    }
}
