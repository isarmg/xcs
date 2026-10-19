//! Missing samples and collection gaps stay gaps in the responsive SVG history.
use super::{Metric, State};
use serde_json::Value;
use wasm_bindgen::{JsCast, closure::Closure};
use xcs_web::{html, i18n::t, prelude::*};

#[derive(Debug)]
struct Plot {
    paths: Vec<String>,
    dots: Vec<(f64, f64)>,
    maximum: f64,
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}
fn plot(
    points: &[(f64, Option<f64>)],
    from: f64,
    to: f64,
    interval: f64,
    width: f64,
    height: f64,
    is_power: bool,
) -> Plot {
    let maximum = if is_power {
        points
            .iter()
            .filter_map(|(_, v)| *v)
            .filter(|v| v.is_finite() && *v >= 0.)
            .fold(1.0f64, f64::max)
            * 1.1
    } else {
        100.
    };
    let left = 38.0f64.min(width * 0.35);
    let right = (width - 5.).max(left + 1.);
    let top = 8.0f64.min(height * 0.2);
    // Short four-column cards retain useful graph height and the same font.
    // Their selected time range is already shown by the toolbar control.
    let bottom = (height - if height < 70. { 6. } else { 24. }).max(top + 1.);
    let mut paths = Vec::new();
    let mut dots = Vec::new();
    let mut segment = Vec::new();
    let mut previous_time = None;
    let flush = |segment: &mut Vec<(f64, f64)>,
                 paths: &mut Vec<String>,
                 dots: &mut Vec<(f64, f64)>| {
        if segment.len() == 1 {
            dots.push(segment[0]);
        } else if !segment.is_empty() {
            paths.push(
                segment
                    .iter()
                    .enumerate()
                    .map(|(i, (x, y))| format!("{}{x:.3},{y:.3}", if i == 0 { "M" } else { "L" }))
                    .collect::<String>(),
            );
        }
        segment.clear();
    };
    for (timestamp, value) in points {
        let value = value.filter(|v| v.is_finite() && *v >= 0. && *v <= maximum);
        if !timestamp.is_finite() || *timestamp < from || *timestamp > to || value.is_none() {
            flush(&mut segment, &mut paths, &mut dots);
            previous_time = None;
            continue;
        }
        if previous_time.is_some_and(|previous| {
            *timestamp <= previous || *timestamp - previous > interval.max(1.) * 2.5
        }) {
            flush(&mut segment, &mut paths, &mut dots);
        }
        let x = left + ((*timestamp - from) / (to - from).max(1.)).clamp(0., 1.) * (right - left);
        let y = bottom - value.unwrap_or_default() / maximum * (bottom - top);
        segment.push((x, y));
        previous_time = Some(*timestamp);
    }
    flush(&mut segment, &mut paths, &mut dots);
    Plot {
        paths,
        dots,
        maximum,
        left,
        right,
        top,
        bottom,
    }
}
fn milliseconds(value: &Value) -> Option<f64> {
    value
        .as_str()
        .map(js_sys::Date::parse)
        .filter(|value| value.is_finite())
}
fn axis_max(value: f64) -> String {
    if value >= 1000. {
        format!("{:.1}k", value / 1000.)
    } else if value >= 10. {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

pub(super) fn view(state: RwSignal<State>, metric: Metric) -> impl IntoView {
    let root = NodeRef::<html::Div>::new();
    let dimensions = RwSignal::new((280.0f64, 110.0f64));
    root.on_load(move |element| {
        let element: web_sys::Element = element.unchecked_into();
        let update = move || {
            let b = element.get_bounding_client_rect();
            let next = (b.width().max(1.), b.height().max(1.));
            if dimensions
                .with_untracked(|old| (old.0 - next.0).abs() > 0.2 || (old.1 - next.1).abs() > 0.2)
            {
                dimensions.set(next);
            }
        };
        update();
        let callback = Closure::wrap(Box::new(
            move |_: js_sys::Array, _: web_sys::ResizeObserver| update(),
        )
            as Box<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>);
        if let Ok(observer) = web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref()) {
            if let Some(element) = root.get_untracked() {
                observer.observe(&element);
            }
            let cleanup = send_wrapper::SendWrapper::new((observer, callback));
            on_cleanup(move || {
                cleanup.0.disconnect();
                drop(cleanup);
            });
        }
    });
    let label = metric.label();
    let aria_label = format!("{} {}", label, t("历史图表", "history chart"));
    let metric = StoredValue::new(metric);
    view! {<div class="xcs-metric-chart" node_ref=root aria-label=aria_label>
        {move ||{
            let (width,height)=dimensions.get();
            let (history,loading,loaded,range)=state.with(|s|(s.monitor.history.clone(),s.monitor.history_loading,s.monitor.history_loaded,s.monitor.range.clone()));
            let points=history["samples"].as_array().map(|samples|samples.iter().filter_map(|sample|Some((milliseconds(&sample["collected_at"])?/1000.,metric.with_value(|metric|metric.value(sample))))).collect::<Vec<_>>()).unwrap_or_default();
            let to=milliseconds(&history["to"]).unwrap_or(0.)/1000.;let from=milliseconds(&history["from"]).unwrap_or(0.)/1000.;
            let interval=history["interval_seconds"].as_f64().unwrap_or(5.);
            let plot=plot(&points,from,to,interval,width,height,metric.with_value(Metric::is_power));
            if plot.paths.is_empty()&&plot.dots.is_empty(){return view!{<p class="xcs-chart-empty">{if loading&&!loaded{t("正在读取历史…","Loading history…")}else{t("暂无可用历史","No available history yet")}}</p>}.into_any();}
            let axis_y=(height-4.).max(14.4);
            view!{<svg viewBox=format!("0 0 {width} {height}") preserveAspectRatio="none" role="img" aria-label=format!("{} · {}",label,range)>
                <title>{format!("{} · {} · {}",label,range,t("空缺表示未采样或不可用，不表示零。","Gaps indicate missing or unavailable samples, not zero."))}</title>
                <path class="xcs-chart-grid" d=format!("M{} {}H{}M{} {}H{}",plot.left,plot.top,plot.right,plot.left,plot.bottom,plot.right)/>
                <text class="xcs-chart-label" x="0" y=(plot.top+5.).to_string()>{axis_max(plot.maximum)}</text>{(height>=42.).then(||view!{<text class="xcs-chart-label" x="0" y=plot.bottom.to_string()>"0"</text>})}
                {(height>=70.).then(||view!{<text class="xcs-chart-label" x=plot.left.to_string() y=axis_y.to_string()>{format!("−{range}")}</text><text class="xcs-chart-label" x=plot.right.to_string() y=axis_y.to_string() text-anchor="end">{t("现在","Now")}</text>})}
                {plot.paths.into_iter().map(|path|view!{<path class="xcs-chart-line" d=path/>}).collect_view()}
                {plot.dots.into_iter().map(|(x,y)|view!{<circle class="xcs-chart-dot" cx=x.to_string() cy=y.to_string() r="2"/>}).collect_view()}
            </svg>}.into_any()
        }}
    </div>}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_plots_preserve_graph_height_and_the_measurement_scale() {
        let p = plot(
            &[(5., Some(20.)), (10., Some(40.))],
            0.,
            60.,
            5.,
            150.,
            36.,
            false,
        );
        assert!(p.bottom - p.top >= 22.0);
        assert_eq!(p.maximum, 100.0);
    }
    #[test]
    fn missing_values_and_collection_gaps_do_not_turn_into_zero_or_continuous_lines() {
        let p = plot(
            &[
                (0., Some(10.)),
                (5., Some(20.)),
                (10., None),
                (15., Some(30.)),
                (50., Some(40.)),
                (55., Some(50.)),
            ],
            0.,
            60.,
            5.,
            300.,
            120.,
            false,
        );
        assert_eq!(p.paths.len(), 2);
        assert_eq!(p.dots.len(), 1);
        assert_eq!(p.maximum, 100.);
        assert!(!p.paths.iter().any(|path| path.contains("NaN")));
    }
    #[test]
    fn power_scales_to_real_measurements_and_single_zero_is_still_a_real_point() {
        let p = plot(
            &[(5., Some(20.)), (10., Some(40.))],
            0.,
            60.,
            5.,
            180.,
            70.,
            true,
        );
        assert_eq!(p.maximum, 44.);
        assert_eq!(p.paths.len(), 1);
        let p = plot(&[(5., Some(0.))], 0., 60., 5., 180., 70., true);
        assert_eq!(p.dots.len(), 1);
        assert!((p.dots[0].1 - p.bottom).abs() < 0.0001);
        let p = plot(&[(5., None)], 0., 60., 5., 180., 70., true);
        assert!(p.paths.is_empty() && p.dots.is_empty());
    }
}
