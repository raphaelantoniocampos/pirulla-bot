//! A chart drawn as SVG, which the reader can point at with the mouse, a finger
//! or the keyboard to see the values of each mark.

use std::sync::Arc;

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{KeyboardEvent, Node, PointerEvent, ResizeObserver};

pub const HEIGHT: f64 = 340.0;
/// On a narrow screen a chart keeps this share of its height.
const NARROW_HEIGHT_RATIO: f64 = 0.82;
/// Below this width a chart drops its direct labels to leave room for the plot.
const NARROW_WIDTH: f64 = 520.0;
const TOOLTIP_GAP: f64 = 14.0;

/// Unlike `f64::clamp`, takes `max` when it is below `min` instead of panicking.
pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

/// The room a chart has, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
    pub narrow: bool,
}

impl Size {
    /// The room of a chart `width` wide that is `height` tall on a wide screen.
    pub fn new(width: f64, height: f64) -> Self {
        let narrow = width < NARROW_WIDTH;
        let height = if narrow {
            (height * NARROW_HEIGHT_RATIO).round()
        } else {
            height
        };
        Self {
            width,
            height,
            narrow,
        }
    }
}

/// A line of a tooltip: the value, what it is, and the mark it goes with in the legend.
pub struct Row {
    pub key: Option<&'static str>,
    pub value: String,
    pub label: String,
}

impl Row {
    pub fn new(key: Option<&'static str>, value: String, label: impl Into<String>) -> Self {
        Self {
            key,
            value,
            label: label.into(),
        }
    }
}

/// What the tooltip of a mark says.
pub struct Tip {
    pub title: String,
    pub rows: Vec<Row>,
    pub note: Option<String>,
}

/// The marks of a chart, drawn for a size, and how the reader points at them.
pub trait Plot: Send + Sync {
    /// How many marks the reader can point at.
    fn count(&self) -> usize;
    /// The mark at the point `x`, `y` of the chart, if any.
    fn index_at(&self, x: f64, y: f64) -> Option<usize>;
    /// Where the tooltip of the mark `index` points.
    fn anchor(&self, index: usize) -> (f64, f64);
    fn describe(&self, index: usize) -> Tip;
    /// The page that the mark `index` opens when clicked, if marks open pages.
    fn link(&self, _index: usize) -> Option<String> {
        None
    }
    /// Whether the marks open pages, so that the pointer shows they can be clicked.
    fn opens_links(&self) -> bool {
        false
    }
    /// Draws the marks; `active` is the one the reader points at.
    fn draw(&self, active: Signal<Option<usize>>) -> AnyView;
}

#[derive(Clone)]
enum Drawing {
    /// The width of the chart is not known yet.
    Unmeasured,
    Empty,
    Plot(Arc<dyn Plot>, Size),
}

/// Keeps a chart drawn by `paint` for the room it has, drawing it again whenever
/// its width or what `paint` reads changes. `paint` answers `None` when there is
/// nothing to draw.
#[component]
pub fn Chart<P>(
    label: &'static str,
    #[prop(default = HEIGHT)] height: f64,
    paint: P,
) -> impl IntoView
where
    P: Fn(Size) -> Option<Arc<dyn Plot>> + Send + Sync + 'static,
{
    let container = NodeRef::<html::Div>::new();
    let tooltip = NodeRef::<html::Div>::new();
    let width = RwSignal::new(0.0);
    let active = RwSignal::new(None::<usize>);
    let transform = RwSignal::new(String::new());
    let pointer_type = StoredValue::new(String::from("mouse"));

    let drawing = Memo::new_with_compare(
        move |_| {
            let width = width.get();
            if width == 0.0 {
                return Drawing::Unmeasured;
            }
            let size = Size::new(width, height);
            match paint(size) {
                Some(plot) => Drawing::Plot(plot, size),
                None => Drawing::Empty,
            }
        },
        // A new drawing is always different from the one before.
        |_, _| true,
    );
    let plot = move || match drawing.get() {
        Drawing::Plot(plot, _) => Some(plot),
        _ => None,
    };

    // What is drawn again has nothing pointed at.
    Effect::new(move |_| {
        drawing.track();
        active.set(None);
    });

    // The chart takes the width of its container, whatever changes it.
    Effect::new(move |_| {
        let Some(element) = container.get() else {
            return;
        };
        width.set(f64::from(element.client_width()));
        let measured = element.clone();
        let observer = Closure::<dyn Fn()>::new(move || {
            let current = f64::from(measured.client_width());
            if width.get_untracked() != current {
                width.set(current);
            }
        });
        if let Ok(resize) = ResizeObserver::new(observer.as_ref().unchecked_ref()) {
            resize.observe(&element);
        }
        // The chart stays on the page for as long as the page is open.
        observer.forget();
    });

    // Puts the tooltip beside the mark, on the side where it fits, without leaving the chart. It is
    // measured in the next frame, once it shows what it says about the mark, but before it is painted.
    Effect::new(move |_| {
        let (Some(index), Some(plot)) = (active.get(), plot()) else {
            return;
        };
        request_animation_frame(move || {
            let (Some(container), Some(tooltip)) =
                (container.get_untracked(), tooltip.get_untracked())
            else {
                return;
            };
            let (x, y) = plot.anchor(index);
            let (tip_width, tip_height) = (
                f64::from(tooltip.offset_width()),
                f64::from(tooltip.offset_height()),
            );
            let room_left = (f64::from(container.client_width()) - tip_width).max(0.0);
            let mut left = x + TOOLTIP_GAP;
            if left > room_left {
                left = x - TOOLTIP_GAP - tip_width;
            }
            let room_top = (f64::from(container.client_height()) - tip_height).max(0.0);
            let top = y - tip_height / 2.0;
            transform.set(format!(
                "translate({}px, {}px)",
                clamp(left, 0.0, room_left),
                clamp(top, 0.0, room_top)
            ));
        });
    });

    let hide = move || active.set(None);
    let open = move |index: usize| {
        if let Some(url) = plot().and_then(|plot| plot.link(index)) {
            let _ = window().open_with_url_and_target_and_features(&url, "_blank", "noopener");
        }
    };
    let on_pointer = move |event: PointerEvent| {
        pointer_type.set_value(event.pointer_type());
        let (Some(plot), Some(container)) = (plot(), container.get()) else {
            return;
        };
        let box_ = container.get_bounding_client_rect();
        let x = f64::from(event.client_x()) - box_.left();
        let y = f64::from(event.client_y()) - box_.top();
        match plot.index_at(x, y) {
            None => hide(),
            Some(index) if active.get_untracked() != Some(index) => active.set(Some(index)),
            Some(_) => {}
        }
    };
    let on_key = move |event: KeyboardEvent| {
        let Some(plot) = plot() else {
            return;
        };
        let last = plot.count() as i64 - 1;
        let current = active.get_untracked().map(|index| index as i64);
        let stride = if event.shift_key() { 10 } else { 1 };
        let next = match event.key().as_str() {
            "ArrowLeft" => current.unwrap_or(last + 1) - stride,
            "ArrowRight" => current.unwrap_or(-1) + stride,
            "Home" => 0,
            "End" => last,
            "Escape" => return hide(),
            "Enter" => {
                if let Some(index) = current {
                    open(index as usize);
                }
                return;
            }
            _ => return,
        };
        event.prevent_default();
        active.set(Some(next.clamp(0, last.max(0)) as usize));
    };

    // A pointer down anywhere else on the page puts the tooltip away.
    let outside = window_event_listener(ev::pointerdown, move |event| {
        let target = event
            .target()
            .and_then(|target| target.dyn_into::<Node>().ok());
        if let Some(container) = container.get_untracked()
            && !container.contains(target.as_ref())
        {
            hide();
        }
    });
    on_cleanup(move || outside.remove());

    view! {
        <div
            class="chart"
            node_ref=container
            tabindex="0"
            role="group"
            aria-label=format!("{label}. Use as setas do teclado para percorrer os valores.")
            on:pointermove=on_pointer
            on:pointerdown=on_pointer
            // Lifting a finger also "leaves" the chart, but what was tapped must stay on screen until the next tap.
            on:pointerleave=move |event| {
                if event.pointer_type() == "mouse" {
                    hide();
                }
            }
            on:blur=move |_| hide()
            on:keydown=on_key
            // A tap only shows the tooltip: on a touch screen the links are in the table.
            on:click=move |_| {
                if pointer_type.get_value() == "mouse" && let Some(index) = active.get_untracked() {
                    open(index);
                }
            }
        >
            {move || match drawing.get() {
                Drawing::Unmeasured => ().into_any(),
                Drawing::Empty => view! { <p class="chart-empty">"Sem vídeos neste período."</p> }.into_any(),
                Drawing::Plot(plot, size) => {
                    let clickable = plot.opens_links();
                    view! {
                        <svg
                            width=size.width
                            height=size.height
                            viewBox=format!("0 0 {} {}", size.width, size.height)
                            aria-hidden="true"
                            style:cursor=move || if clickable && active.get().is_some() { "pointer" } else { "" }
                        >
                            {plot.draw(active.into())}
                        </svg>
                    }
                        .into_any()
                }
            }}
            <div class="tooltip" node_ref=tooltip hidden=move || active.get().is_none() style:transform=move || transform.get()>
                {move || active.get().zip(plot()).map(|(index, plot)| tip(plot.describe(index)))}
            </div>
        </div>
    }
}

/// The titles come from YouTube, so everything goes in as text, never as HTML.
fn tip(Tip { title, rows, note }: Tip) -> impl IntoView {
    view! {
        <p class="tooltip-title">{title}</p>
        {rows
            .into_iter()
            .map(|row| {
                view! {
                    <p class="tooltip-row">
                        {row.key.map(|key| view! { <span class=format!("key {key}")></span> })}
                        <strong>{row.value}</strong>
                        " "
                        <span class="tooltip-label">{row.label}</span>
                    </p>
                }
            })
            .collect_view()}
        {note.map(|note| view! { <p class="tooltip-note">{note}</p> })}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_a_value_into_a_range() {
        assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
        assert_eq!(clamp(-1.0, 0.0, 10.0), 0.0);
        assert_eq!(clamp(11.0, 0.0, 10.0), 10.0);
    }

    #[test]
    fn clamping_into_a_range_too_small_takes_its_end() {
        assert_eq!(clamp(5.0, 10.0, 0.0), 0.0);
    }

    #[test]
    fn a_wide_chart_keeps_its_height() {
        assert_eq!(
            Size::new(966.0, 340.0),
            Size {
                width: 966.0,
                height: 340.0,
                narrow: false
            }
        );
        assert!(!Size::new(520.0, 340.0).narrow);
    }

    #[test]
    fn a_narrow_chart_is_shorter() {
        assert_eq!(
            Size::new(414.0, 340.0),
            Size {
                width: 414.0,
                height: 279.0,
                narrow: true
            }
        );
        assert_eq!(Size::new(519.0, 420.0).height, 344.0);
    }
}
