//! The three charts of the page: the rate over time, the duration of every video
//! and the average of each period.

use leptos::prelude::*;
use pirulla_core::format::{
    format_count, format_date, format_duration, format_duration_ms, format_duration_short,
    format_kind, format_percent, format_total, format_variation,
};
use pirulla_core::scale::{Linear, duration_ticks, nearest_index, time_ticks};
use pirulla_core::stats::{PeriodGroup, RatedVideo, millis};

use crate::chart::{Plot, Row, Size, Tip, clamp};

/// How far, in pixels, the pointer can be from a dot and still select it.
const HOVER_RADIUS: f64 = 28.0;

/// The area of the chart the marks are drawn in, leaving room for the labels around it.
#[derive(Clone, Copy)]
struct Area {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl Area {
    fn new(size: Size, y_ticks: &[f64], right_margin: f64) -> Self {
        let longest = y_ticks
            .iter()
            .map(|tick| format_duration_short(*tick).chars().count())
            .max()
            .unwrap_or(1)
            .max(1);
        Self {
            left: (longest as f64 * 6.6 + 16.0).round(),
            right: size.width - right_margin,
            top: 14.0,
            bottom: size.height - 30.0,
        }
    }
}

struct XTick {
    x: f64,
    label: String,
}

/// The grid, the labels of the ticks and the base line.
fn axes(area: Area, y: Linear, y_ticks: &[f64], x_ticks: Vec<XTick>) -> impl IntoView {
    let baseline = area.bottom.round() + 0.5;
    let rows: Vec<(f64, String)> = y_ticks
        .iter()
        .map(|tick| (y.at(*tick).round() + 0.5, format_duration_short(*tick)))
        .collect();
    view! {
        <g class="grid">
            {rows
                .iter()
                .map(|(position, _)| view! { <line x1=area.left x2=area.right y1=*position y2=*position /> })
                .collect_view()}
        </g>
        <line class="axis" x1=area.left x2=area.right y1=baseline y2=baseline />
        <g class="tick-labels">
            {rows
                .into_iter()
                .map(|(position, label)| {
                    view! {
                        <text x=area.left - 8.0 y=position dy="0.32em" text-anchor="end">
                            {label}
                        </text>
                    }
                })
                .collect_view()}
            {x_ticks
                .into_iter()
                .map(|tick| {
                    view! {
                        <text x=tick.x y=area.bottom + 19.0 text-anchor="middle">
                            {tick.label}
                        </text>
                    }
                })
                .collect_view()}
        </g>
    }
}

fn time_axis(times: &[f64], x: Linear, area: Area) -> Vec<XTick> {
    let max_ticks = ((area.right - area.left) / 80.0).floor().max(2.0) as usize;
    let (Some(first), Some(last)) = (times.first(), times.last()) else {
        return Vec::new();
    };
    time_ticks(*first, *last, max_ticks)
        .into_iter()
        .map(|tick| XTick {
            x: x.at(tick.time),
            label: tick.label,
        })
        .collect()
}

fn line_path(points: impl Iterator<Item = (f64, f64)>) -> String {
    let points: Vec<String> = points.map(|(x, y)| format!("{x:.1},{y:.1}")).collect();
    format!("M{}", points.join("L"))
}

fn visibility(visible: bool) -> &'static str {
    if visible { "visible" } else { "hidden" }
}

/// The average duration over time, as a line.
pub struct RatePlot {
    videos: Vec<RatedVideo>,
    times: Vec<f64>,
    size: Size,
    area: Area,
    x: Linear,
    y: Linear,
    y_ticks: Vec<f64>,
}

impl RatePlot {
    /// `None` when there are no videos.
    pub fn new(videos: Vec<RatedVideo>, size: Size) -> Option<Self> {
        let times: Vec<f64> = videos
            .iter()
            .map(|video| millis(video.video.published_at))
            .collect();
        let lowest = videos.iter().map(|video| video.mean).reduce(f64::min)?;
        let highest = videos.iter().map(|video| video.mean).reduce(f64::max)?;
        let spread = highest - lowest;
        let padding = if spread == 0.0 { 1.0 } else { spread } * 0.08;
        let domain = (lowest - padding, highest + padding);
        let y_ticks = duration_ticks(domain.0, domain.1, if size.narrow { 5 } else { 6 });
        let area = Area::new(size, &y_ticks, if size.narrow { 16.0 } else { 104.0 });
        let x = Linear::new((times[0], times[times.len() - 1]), (area.left, area.right));
        let y = Linear::new(domain, (area.bottom, area.top));
        Some(Self {
            videos,
            times,
            size,
            area,
            x,
            y,
            y_ticks,
        })
    }
}

impl Plot for RatePlot {
    fn count(&self) -> usize {
        self.videos.len()
    }

    fn index_at(&self, x: f64, _y: f64) -> Option<usize> {
        if x < self.area.left - 12.0 || x > self.area.right + 12.0 {
            return None;
        }
        nearest_index(&self.times, self.x.invert(x))
    }

    fn anchor(&self, index: usize) -> (f64, f64) {
        (
            self.x.at(self.times[index]),
            self.y.at(self.videos[index].mean),
        )
    }

    fn describe(&self, index: usize) -> Tip {
        let video = &self.videos[index];
        Tip {
            title: format_date(video.video.published_at),
            rows: vec![
                Row::new(Some("line s1"), format_duration_ms(video.mean), "Cotação"),
                Row::new(
                    None,
                    format!(
                        "{} ({})",
                        format_variation(video.variation),
                        format_percent(video.variation_percent)
                    ),
                    "com este vídeo",
                ),
            ],
            note: Some(format!(
                "{} · {}",
                video.video.title,
                format_duration(video.video.duration as f64)
            )),
        }
    }

    fn draw(&self, active: Signal<Option<usize>>) -> AnyView {
        let (area, x, y) = (self.area, self.x, self.y);
        let path = line_path(
            self.times
                .iter()
                .zip(&self.videos)
                .map(|(time, video)| (x.at(*time), y.at(video.mean))),
        );
        let latest = self.videos.len() - 1;
        let (latest_x, latest_y) = self.anchor(latest);
        let label = (!self.size.narrow).then(|| {
            view! {
                <text class="direct-label" x=latest_x + 10.0 y=latest_y dy="0.32em">
                    {format_duration(self.videos[latest].mean)}
                </text>
            }
        });
        let anchors: Vec<(f64, f64)> = (0..self.videos.len())
            .map(|index| self.anchor(index))
            .collect();
        let anchors = StoredValue::new(anchors);
        let at = move |index: Option<usize>| {
            index.map(|index| anchors.with_value(|anchors| anchors[index]))
        };
        let crosshair = move || at(active.get()).map_or(0.0, |(x, _)| x.round() + 0.5);
        view! {
            {axes(area, y, &self.y_ticks, time_axis(&self.times, x, area))}
            <path class="line s1" d=path />
            <circle class="dot s1" cx=latest_x cy=latest_y r="4" />
            {label}
            <line
                class="crosshair"
                y1=area.top
                y2=area.bottom
                x1=crosshair
                x2=crosshair
                visibility=move || visibility(active.get().is_some())
            />
            <circle
                class="dot s1 is-active"
                r="5"
                cx=move || at(active.get()).map_or(0.0, |(x, _)| x)
                cy=move || at(active.get()).map_or(0.0, |(_, y)| y)
                visibility=move || visibility(active.get().is_some())
            />
        }
        .into_any()
    }
}

/// The duration of every video, as dots, with the average over time as a line.
pub struct VideosPlot {
    videos: Vec<RatedVideo>,
    times: Vec<f64>,
    positions: Vec<(f64, f64)>,
    size: Size,
    area: Area,
    x: Linear,
    y: Linear,
    y_ticks: Vec<f64>,
}

impl VideosPlot {
    /// `None` when there are no videos.
    pub fn new(videos: Vec<RatedVideo>, size: Size) -> Option<Self> {
        let longest = videos.iter().map(|video| video.video.duration).max()?;
        let times: Vec<f64> = videos
            .iter()
            .map(|video| millis(video.video.published_at))
            .collect();
        let domain = (0.0, longest as f64 * 1.05);
        let y_ticks = duration_ticks(0.0, domain.1, if size.narrow { 5 } else { 6 });
        let area = Area::new(size, &y_ticks, if size.narrow { 16.0 } else { 84.0 });
        // Inset, so that the first and the last dots are not cut by the edges.
        let x = Linear::new(
            (times[0], times[times.len() - 1]),
            (area.left + 6.0, area.right - 6.0),
        );
        let y = Linear::new(domain, (area.bottom, area.top));
        let positions = times
            .iter()
            .zip(&videos)
            .map(|(time, video)| (x.at(*time), y.at(video.video.duration as f64)))
            .collect();
        Some(Self {
            videos,
            times,
            positions,
            size,
            area,
            x,
            y,
            y_ticks,
        })
    }

    fn radius(&self) -> f64 {
        if self.size.narrow { 2.5 } else { 3.5 }
    }
}

impl Plot for VideosPlot {
    fn count(&self) -> usize {
        self.videos.len()
    }

    fn index_at(&self, x: f64, y: f64) -> Option<usize> {
        let mut nearest = None;
        let mut nearest_distance = HOVER_RADIUS.powi(2);
        for (index, (dot_x, dot_y)) in self.positions.iter().enumerate() {
            let distance = (dot_x - x).powi(2) + (dot_y - y).powi(2);
            if distance < nearest_distance {
                (nearest, nearest_distance) = (Some(index), distance);
            }
        }
        nearest
    }

    fn anchor(&self, index: usize) -> (f64, f64) {
        self.positions[index]
    }

    fn describe(&self, index: usize) -> Tip {
        let video = &self.videos[index];
        let note = [
            format_date(video.video.published_at),
            format_kind(video.video.kind).to_string(),
        ];
        Tip {
            title: video.video.title.clone(),
            rows: vec![
                Row::new(
                    Some("dot s2"),
                    format_duration(video.video.duration as f64),
                    "Duração",
                ),
                Row::new(
                    Some("line s1"),
                    format_duration(video.mean),
                    "Cotação na data",
                ),
            ],
            note: Some(
                note.into_iter()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · "),
            ),
        }
    }

    fn link(&self, index: usize) -> Option<String> {
        Some(self.videos[index].video.url())
    }

    fn opens_links(&self) -> bool {
        true
    }

    fn draw(&self, active: Signal<Option<usize>>) -> AnyView {
        let (area, x, y) = (self.area, self.x, self.y);
        let radius = self.radius();
        let path = line_path(
            self.times
                .iter()
                .zip(&self.videos)
                .map(|(time, video)| (x.at(*time), y.at(video.mean))),
        );
        let label = (!self.size.narrow).then(|| {
            let latest = &self.videos[self.videos.len() - 1];
            let label_x = x.at(self.times[self.times.len() - 1]) + 12.0;
            view! {
                <text class="direct-label" x=label_x y=y.at(latest.mean) dy="0.32em">
                    "Cotação"
                </text>
            }
        });
        let positions = StoredValue::new(self.positions.clone());
        let at = move |index: Option<usize>| {
            index.map(|index| positions.with_value(|positions| positions[index]))
        };
        view! {
            {axes(area, y, &self.y_ticks, time_axis(&self.times, x, area))}
            <g class="dots">
                {self
                    .positions
                    .iter()
                    .map(|(dot_x, dot_y)| {
                        view! { <circle cx=format!("{dot_x:.1}") cy=format!("{dot_y:.1}") r=radius /> }
                    })
                    .collect_view()}
            </g>
            <path class="line-casing" d=path.clone() />
            <path class="line s1" d=path />
            {label}
            <circle
                class="dot s2 is-active"
                r=radius + 2.5
                cx=move || at(active.get()).map_or(0.0, |(x, _)| x)
                cy=move || at(active.get()).map_or(0.0, |(_, y)| y)
                visibility=move || visibility(active.get().is_some())
            />
        }
        .into_any()
    }
}

/// The average duration of the videos of each year or month, as columns.
pub struct PeriodsPlot {
    groups: Vec<PeriodGroup>,
    size: Size,
    area: Area,
    y: Linear,
    y_ticks: Vec<f64>,
    band: f64,
    bar_width: f64,
}

impl PeriodsPlot {
    /// `None` when there are no periods.
    pub fn new(groups: Vec<PeriodGroup>, size: Size) -> Option<Self> {
        if groups.is_empty() {
            return None;
        }
        let highest = groups
            .iter()
            .filter_map(|group| group.mean)
            .fold(0.0, f64::max);
        let highest = if highest == 0.0 { 1.0 } else { highest };
        let domain = (0.0, highest * 1.12);
        let y_ticks = duration_ticks(0.0, domain.1, if size.narrow { 4 } else { 5 });
        let area = Area::new(size, &y_ticks, 12.0);
        let y = Linear::new(domain, (area.bottom, area.top));
        let band = (area.right - area.left) / groups.len() as f64;
        // Never wider than 24px, and always 2px apart from the neighbours.
        let bar_width = clamp(band - 2.0, 1.0, 24.0);
        Some(Self {
            groups,
            size,
            area,
            y,
            y_ticks,
            band,
            bar_width,
        })
    }

    fn center(&self, index: usize) -> f64 {
        self.area.left + self.band * (index as f64 + 0.5)
    }

    /// The columns worth a label: the highest, and the most recent when the column before it leaves room.
    fn labelled(&self) -> Vec<usize> {
        let mut labelled = Vec::new();
        let mut highest: Option<usize> = None;
        for (index, group) in self.groups.iter().enumerate() {
            if let Some(mean) = group.mean
                && highest.is_none_or(|highest| mean > self.groups[highest].mean.unwrap_or(0.0))
            {
                highest = Some(index);
            }
        }
        labelled.extend(highest);
        let latest = self.groups.len() - 1;
        let before = if latest > 0 {
            self.groups[latest - 1].mean.unwrap_or(0.0)
        } else {
            0.0
        };
        if let Some(mean) = self.groups[latest].mean
            && before <= mean
            && !labelled.contains(&latest)
        {
            labelled.push(latest);
        }
        labelled
    }
}

/// A column with its top corners rounded and its base square.
fn column_path(left: f64, top: f64, width: f64, bottom: f64) -> String {
    let radius = 4.0f64.min(width / 2.0).min(bottom - top).max(0.0);
    let right = left + width;
    format!(
        "M{left},{bottom}V{}Q{left},{top} {},{top}H{}Q{right},{top} {right},{}V{bottom}Z",
        top + radius,
        left + radius,
        right - radius,
        top + radius
    )
}

/// Whole minutes for long averages, so that the label stays short.
fn column_label(mean: f64) -> String {
    format_duration_short(if mean >= 600.0 {
        (mean / 60.0).round() * 60.0
    } else {
        mean.round()
    })
}

impl Plot for PeriodsPlot {
    fn count(&self) -> usize {
        self.groups.len()
    }

    fn index_at(&self, x: f64, _y: f64) -> Option<usize> {
        if x < self.area.left || x >= self.area.right {
            return None;
        }
        Some((((x - self.area.left) / self.band).floor() as usize).min(self.groups.len() - 1))
    }

    fn anchor(&self, index: usize) -> (f64, f64) {
        let y = match self.groups[index].mean {
            Some(mean) => self.y.at(mean),
            None => (self.area.top + self.area.bottom) / 2.0,
        };
        (self.center(index) + self.bar_width / 2.0, y)
    }

    fn describe(&self, index: usize) -> Tip {
        let group = &self.groups[index];
        let rows = match group.mean {
            None => vec![Row::new(None, "Nenhum".to_string(), "vídeo publicado")],
            Some(mean) => vec![
                Row::new(Some("box s1"), format_duration(mean), "Duração média"),
                Row::new(
                    None,
                    format_count(group.count as u64),
                    if group.count == 1 {
                        "vídeo"
                    } else {
                        "vídeos"
                    },
                ),
                Row::new(None, format_total(group.total), "no total"),
            ],
        };
        Tip {
            title: group.label.clone(),
            rows,
            note: None,
        }
    }

    fn draw(&self, active: Signal<Option<usize>>) -> AnyView {
        let (area, y) = (self.area, self.y);
        // Only as many labels as fit, counted back from the most recent period.
        let every_nth = ((self.groups[0].label.chars().count() as f64 * 7.0 + 12.0) / self.band)
            .ceil()
            .max(1.0) as usize;
        let last = self.groups.len() - 1;
        let x_ticks = self
            .groups
            .iter()
            .enumerate()
            .filter(|(index, _)| (last - index).is_multiple_of(every_nth))
            .map(|(index, group)| XTick {
                x: self.center(index),
                label: group.label.clone(),
            })
            .collect();
        let bars = self
            .groups
            .iter()
            .enumerate()
            .filter_map(|(index, group)| {
                let mean = group.mean?;
                let d = column_path(self.center(index) - self.bar_width / 2.0, y.at(mean), self.bar_width, area.bottom);
                Some(view! { <path class="bar" class:is-active=move || active.get() == Some(index) d=d /> })
            })
            .collect_view();
        let labels = self
            .labelled()
            .into_iter()
            .map(|index| {
                let mean = self.groups[index].mean.unwrap_or(0.0);
                let text = column_label(mean);
                let half_width = text.chars().count() as f64 * 3.4;
                let position = clamp(
                    self.center(index),
                    area.left + half_width,
                    self.size.width - half_width - 2.0,
                );
                view! {
                    <text class="direct-label" x=position y=y.at(mean) - 7.0 text-anchor="middle">
                        {text}
                    </text>
                }
            })
            .collect_view();
        view! {
            {axes(area, y, &self.y_ticks, x_ticks)}
            {bars}
            {labels}
        }
        .into_any()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pirulla_core::data::{Video, VideoKind};
    use pirulla_core::stats::{Period, group_by_period, with_rate};

    const WIDE: Size = Size {
        width: 966.0,
        height: 340.0,
        narrow: false,
    };

    fn video(id: &str, published_at: &str, duration: u64, kind: VideoKind) -> Video {
        Video {
            id: id.to_string(),
            title: format!("Vídeo {id}"),
            published_at: published_at.parse().unwrap(),
            duration,
            kind,
        }
    }

    /// Three videos, a year apart: 10 minutes, 20 minutes and a 1-minute live.
    fn videos() -> Vec<RatedVideo> {
        with_rate(&[
            video("a", "2022-01-01T12:00:00Z", 600, VideoKind::Video),
            video("b", "2023-01-01T12:00:00Z", 1200, VideoKind::Video),
            video("c", "2024-01-01T12:00:00Z", 60, VideoKind::Live),
        ])
    }

    fn rows(tip: &Tip) -> Vec<(Option<&str>, &str, &str)> {
        tip.rows
            .iter()
            .map(|row| (row.key, row.value.as_str(), row.label.as_str()))
            .collect()
    }

    #[test]
    fn the_plot_leaves_room_for_the_longest_label_of_the_axis() {
        let area = Area::new(WIDE, &[0.0, 1800.0, 5400.0], 104.0);

        // "30min" is the longest label: 5 characters.
        assert_eq!(area.left, (5.0 * 6.6 + 16.0f64).round());
        assert_eq!((area.right, area.top, area.bottom), (862.0, 14.0, 310.0));
    }

    #[test]
    fn nothing_is_drawn_without_videos() {
        assert!(RatePlot::new(Vec::new(), WIDE).is_none());
        assert!(VideosPlot::new(Vec::new(), WIDE).is_none());
        assert!(PeriodsPlot::new(Vec::new(), WIDE).is_none());
    }

    #[test]
    fn the_rate_line_runs_from_the_first_video_to_the_last() {
        let plot = RatePlot::new(videos(), WIDE).unwrap();

        assert_eq!(plot.count(), 3);
        assert_eq!(plot.anchor(0).0, plot.area.left);
        assert_eq!(plot.anchor(2).0, plot.area.right);
        // A higher average is drawn higher up.
        assert!(plot.anchor(1).1 < plot.anchor(0).1);
    }

    #[test]
    fn the_rate_chart_points_at_the_video_nearest_in_time() {
        let plot = RatePlot::new(videos(), WIDE).unwrap();
        let (middle, _) = plot.anchor(1);

        assert_eq!(plot.index_at(middle - 5.0, 0.0), Some(1));
        assert_eq!(plot.index_at(plot.area.left - 12.0, 300.0), Some(0));
        assert_eq!(plot.index_at(plot.area.right + 12.0, 300.0), Some(2));
        assert_eq!(plot.index_at(plot.area.left - 13.0, 100.0), None);
        assert_eq!(plot.index_at(plot.area.right + 13.0, 100.0), None);
    }

    #[test]
    fn the_rate_chart_tells_the_rate_at_each_video() {
        let plot = RatePlot::new(videos(), WIDE).unwrap();

        let tip = plot.describe(1);

        assert_eq!(tip.title, "01/01/2023");
        assert_eq!(
            rows(&tip),
            [
                (Some("line s1"), "15min 00s 000ms", "Cotação"),
                (None, "+5min 00s (+50,00%)", "com este vídeo"),
            ]
        );
        assert_eq!(tip.note.as_deref(), Some("Vídeo b · 20min 00s"));
        assert_eq!(plot.link(1), None);
        assert!(!plot.opens_links());
    }

    #[test]
    fn the_dots_are_found_near_the_pointer_and_open_their_video() {
        let plot = VideosPlot::new(videos(), WIDE).unwrap();
        let (x, y) = plot.anchor(1);

        assert_eq!(plot.index_at(x + 10.0, y - 10.0), Some(1));
        assert_eq!(plot.index_at(x + 20.0, y + 20.0), None);
        assert_eq!(
            plot.link(1).as_deref(),
            Some("https://www.youtube.com/watch?v=b")
        );
        assert!(plot.opens_links());
    }

    #[test]
    fn the_longest_video_reaches_near_the_top_of_the_chart() {
        let plot = VideosPlot::new(videos(), WIDE).unwrap();

        let (_, top) = plot.anchor(1);

        assert!(top > plot.area.top && top < plot.area.top + 20.0, "{top}");
        assert_eq!(plot.anchor(0).0, plot.area.left + 6.0);
        assert_eq!(plot.anchor(2).0, plot.area.right - 6.0);
    }

    #[test]
    fn a_dot_tells_its_video_and_the_rate_at_the_time() {
        let plot = VideosPlot::new(videos(), WIDE).unwrap();

        let tip = plot.describe(2);

        assert_eq!(tip.title, "Vídeo c");
        assert_eq!(
            rows(&tip),
            [
                (Some("dot s2"), "1min 00s", "Duração"),
                (Some("line s1"), "10min 20s", "Cotação na data")
            ]
        );
        assert_eq!(tip.note.as_deref(), Some("01/01/2024 · live"));
        assert_eq!(plot.describe(0).note.as_deref(), Some("01/01/2022"));
    }

    #[test]
    fn narrow_dots_are_smaller() {
        let narrow = Size {
            width: 414.0,
            height: 344.0,
            narrow: true,
        };

        assert_eq!(VideosPlot::new(videos(), narrow).unwrap().radius(), 2.5);
        assert_eq!(VideosPlot::new(videos(), WIDE).unwrap().radius(), 3.5);
    }

    fn periods() -> PeriodsPlot {
        let groups = group_by_period(
            &with_rate(&[
                video("a", "2021-06-01T12:00:00Z", 600, VideoKind::Video),
                video("b", "2023-06-01T12:00:00Z", 1800, VideoKind::Video),
                video("c", "2024-06-01T12:00:00Z", 1200, VideoKind::Video),
                video("d", "2024-07-01T12:00:00Z", 1500, VideoKind::Video),
            ]),
            Period::Year,
        );
        PeriodsPlot::new(groups, WIDE).unwrap()
    }

    #[test]
    fn each_period_has_a_band_of_the_chart() {
        let plot = periods();
        let left = plot.area.left;

        assert_eq!(plot.count(), 4);
        assert_eq!(plot.index_at(left, 100.0), Some(0));
        assert_eq!(plot.index_at(left + plot.band * 2.5, 100.0), Some(2));
        assert_eq!(plot.index_at(plot.area.right - 0.1, 100.0), Some(3));
        assert_eq!(plot.index_at(left - 0.1, 100.0), None);
        assert_eq!(plot.index_at(plot.area.right, 100.0), None);
    }

    #[test]
    fn a_column_is_never_wider_than_24_pixels() {
        assert_eq!(periods().bar_width, 24.0);
    }

    #[test]
    fn a_period_tells_its_average_count_and_total() {
        let plot = periods();

        let busy = plot.describe(3);
        let single = plot.describe(0);
        let empty = plot.describe(1);

        assert_eq!(busy.title, "2024");
        assert_eq!(
            rows(&busy),
            [
                (Some("box s1"), "22min 30s", "Duração média"),
                (None, "2", "vídeos"),
                (None, "0h 45min", "no total"),
            ]
        );
        assert_eq!(rows(&single)[1], (None, "1", "vídeo"));
        assert_eq!(empty.title, "2022");
        assert_eq!(rows(&empty), [(None, "Nenhum", "vídeo publicado")]);
    }

    #[test]
    fn the_tooltip_of_an_empty_period_points_at_the_middle_of_the_chart() {
        let plot = periods();

        assert_eq!(plot.anchor(1).1, (plot.area.top + plot.area.bottom) / 2.0);
        assert_eq!(plot.anchor(0).0, plot.center(0) + 12.0);
    }

    #[test]
    fn the_highest_column_and_the_latest_are_labelled() {
        // 2023 is the highest; 2024 is lower than 2023, so it is not labelled.
        assert_eq!(periods().labelled(), [2]);

        let rising = PeriodsPlot::new(
            group_by_period(
                &with_rate(&[
                    video("a", "2021-06-01T12:00:00Z", 600, VideoKind::Video),
                    video("b", "2022-06-01T12:00:00Z", 900, VideoKind::Video),
                    video("c", "2023-06-01T12:00:00Z", 1200, VideoKind::Video),
                ]),
                Period::Year,
            ),
            WIDE,
        )
        .unwrap();
        // The latest is also the highest, and is labelled once.
        assert_eq!(rising.labelled(), [2]);
    }

    #[test]
    fn a_column_has_rounded_top_corners() {
        assert_eq!(
            column_path(10.0, 50.0, 20.0, 100.0),
            "M10,100V54Q10,50 14,50H26Q30,50 30,54V100Z"
        );
        // Too thin or too short for the full rounding.
        assert_eq!(
            column_path(0.0, 98.0, 2.0, 100.0),
            "M0,100V99Q0,98 1,98H1Q2,98 2,99V100Z"
        );
    }

    #[test]
    fn long_averages_are_labelled_in_whole_minutes() {
        assert_eq!(column_label(1513.0), "25min");
        assert_eq!(column_label(599.4), "9min 59s");
        assert_eq!(column_label(45.6), "46s");
    }
}
