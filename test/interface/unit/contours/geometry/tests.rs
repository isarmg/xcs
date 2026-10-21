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
        let index = ((t * REFERENCE_INTERVALS as f64).floor() as usize).min(REFERENCE_INTERVALS);
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
                let t =
                    a.parameter.unwrap() + (b.parameter.unwrap() - a.parameter.unwrap()) * fraction;
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
