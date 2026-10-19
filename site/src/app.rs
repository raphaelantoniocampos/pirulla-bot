//! Loads the data written by the generator and fills the page with it.

use std::sync::Arc;

use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use pirulla_core::data::{SiteData, Video, VideoKind};
use pirulla_core::format::{
    format_count, format_date, format_date_time, format_duration, format_duration_ms, format_kind,
    format_percent, format_total, format_variation,
};
use pirulla_core::stats::{
    DEFAULT_KINDS, Kinds, Period, RatedVideo, YEAR_MS, group_by_period, last_years, millis,
    of_kinds, summarize, with_rate,
};
use pirulla_core::tweet::{MAX_TWEET_LENGTH, compose_tweet, intent_url, weighted_length};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{RequestCache, RequestInit, Response};

use crate::chart::{Chart, Plot};
use crate::plots::{PeriodsPlot, RatePlot, VideosPlot};

const DATA_URL: &str = "data.json";
/// Spans shorter than this are shown month by month instead of year by year.
const MONTHLY_SPAN: f64 = 2.5 * YEAR_MS;
const FIRST_ROWS: usize = 15;
const MORE_ROWS: usize = 50;

async fn load() -> Result<SiteData, String> {
    let failed = |error: wasm_bindgen::JsValue| format!("{error:?}");
    let init = RequestInit::new();
    init.set_cache(RequestCache::NoCache);
    let response: Response = JsFuture::from(window().fetch_with_str_and_init(DATA_URL, &init))
        .await
        .map_err(failed)?
        .dyn_into()
        .map_err(failed)?;
    if !response.ok() {
        return Err(format!("{DATA_URL} answered {}", response.status()));
    }
    let text = JsFuture::from(response.text().map_err(failed)?)
        .await
        .map_err(failed)?;
    let data: SiteData = serde_json::from_str(&text.as_string().unwrap_or_default())
        .map_err(|error| error.to_string())?;
    if data.videos.is_empty() {
        return Err("the data file has no videos".to_string());
    }
    Ok(data)
}

#[component]
pub fn App() -> impl IntoView {
    let data = RwSignal::new(None::<Result<Arc<SiteData>, String>>);
    let tweet = RwSignal::new(String::new());
    let dialog = NodeRef::<html::Dialog>::new();
    spawn_local(async move {
        let loaded = load().await.map(Arc::new);
        if let Err(error) = &loaded {
            leptos::logging::error!("{error}");
        }
        data.set(Some(loaded));
    });
    let channel = move || match data.get() {
        Some(Ok(data)) => format!(
            "https://www.youtube.com/channel/{}",
            encode(&data.channel_id)
        ),
        _ => "https://www.youtube.com/".to_string(),
    };

    view! {
        <main>
            <header class="masthead">
                <h1>"Cotação do Pirulla"</h1>
                <p class="lede">
                    "O " <strong>"Pirulla"</strong> " é a unidade de tempo que equivale à duração média dos vídeos do "
                    <a id="channel-link" href=channel target="_blank" rel="noopener">
                        "canal do Pirulla"
                    </a> ". A cada vídeo novo, a cotação muda."
                </p>
            </header>
            {move || match data.get() {
                None => ().into_any(),
                Some(Err(_)) => {
                    view! {
                        <p id="status" class="status" role="status">
                            "Não foi possível carregar os dados agora. Tente recarregar a página em alguns minutos."
                        </p>
                    }
                        .into_any()
                }
                Some(Ok(data)) => view! { <Dashboard data tweet dialog /> }.into_any(),
            }}
            <footer>
                <p id="updated">
                    {move || match data.get() {
                        Some(Ok(data)) => format!("Atualizado em {}.", format_date_time(data.generated_at)),
                        _ => String::new(),
                    }}
                </p>
                <p>
                    "Dados da YouTube Data API. Site não oficial, sem vínculo com o canal. "
                    <a href="https://github.com/raphaelantoniocampos/pirulla-bot" target="_blank" rel="noopener">
                        "Código no GitHub"
                    </a> "."
                </p>
            </footer>
        </main>
        // Outside of the main column, which would take its margins and with them its place in the middle of the screen.
        <TweetDialog dialog text=tweet />
    }
}

/// Percent-encodes a part of an address, as `encodeURIComponent` does.
fn encode(text: &str) -> String {
    js_sys::encode_uri_component(text).into()
}

/// The page once the data is in. What counts scopes the whole page; the period, the charts and the table below it.
/// "Gerar tweet" writes the post about the latest video that counts in `tweet` and opens `dialog` with it.
#[component]
fn Dashboard(
    data: Arc<SiteData>,
    tweet: RwSignal<String>,
    dialog: NodeRef<html::Dialog>,
) -> impl IntoView {
    let now = data.generated_at;
    let videos = StoredValue::new(data.videos.clone());
    let lives = RwSignal::new(DEFAULT_KINDS.lives);
    let shorts = RwSignal::new(DEFAULT_KINDS.shorts);
    let range = RwSignal::new(None::<u32>);

    let counted = Memo::new(move |_| {
        let kinds = Kinds {
            lives: lives.get(),
            shorts: shorts.get(),
        };
        videos.with_value(|videos| with_rate(&of_kinds(videos, kinds)))
    });
    let selected =
        Memo::new(move |_| counted.with(|counted| last_years(counted, range.get(), now)));
    let period = Memo::new(move |_| selected.with(|selected| period_of(selected)));
    let groups =
        Memo::new(move |_| selected.with(|selected| group_by_period(selected, period.get())));
    let open_tweet = move |_| {
        if let Some(latest) = counted.with(|counted| counted.last().cloned()) {
            tweet.set(compose_tweet(&latest, &site_url()));
            if let Some(dialog) = dialog.get() {
                let _ = dialog.show_modal();
            }
        }
    };

    view! {
        <div class="filters">
            <span id="kinds-label" class="label">
                "Além dos vídeos, entram na cotação"
            </span>
            <div class="options toggles" role="group" aria-labelledby="kinds-label">
                <Toggle id="count-lives" label="Lives" on=lives />
                <Toggle id="count-shorts" label="Shorts" on=shorts />
            </div>
        </div>

        <Summary counted open_tweet />

        <div class="filters">
            <span id="range-label" class="label">
                "Período"
            </span>
            <div class="options" role="radiogroup" aria-labelledby="range-label">
                {[(Some(1), "1 ano"), (Some(5), "5 anos"), (Some(10), "10 anos"), (None, "Tudo")]
                    .into_iter()
                    .map(|(years, label)| {
                        view! {
                            <label>
                                <input
                                    type="radio"
                                    name="range"
                                    value=years.map_or("all".to_string(), |years| years.to_string())
                                    prop:checked=move || range.get() == years
                                    on:change=move |_| range.set(years)
                                />
                                <span>{label}</span>
                            </label>
                        }
                    })
                    .collect_view()}
            </div>
        </div>

        <figure class="card">
            <figcaption>
                <h2>"Cotação ao longo do tempo"</h2>
                <p>"Duração média dos vídeos publicados até cada data."</p>
            </figcaption>
            <Chart
                label="Gráfico da cotação ao longo do tempo"
                paint=move |size| selected.with(|selected| RatePlot::new(selected.clone(), size)).map(|plot| Arc::new(plot) as Arc<dyn Plot>)
            />
            <p class="table-link">
                <a href="#videos">"Ver como tabela"</a>
            </p>
        </figure>

        <figure class="card">
            <figcaption>
                <h2>"Duração de cada vídeo"</h2>
                <p>"Cada ponto é um vídeo. Com o mouse, clique em um ponto para abrir o vídeo no YouTube."</p>
                <ul class="legend">
                    <li>
                        <span class="key dot s2"></span>
                        "Vídeo"
                    </li>
                    <li>
                        <span class="key line s1"></span>
                        "Cotação"
                    </li>
                </ul>
            </figcaption>
            <Chart
                label="Gráfico da duração de cada vídeo"
                height=420.0
                paint=move |size| selected.with(|selected| VideosPlot::new(selected.clone(), size)).map(|plot| Arc::new(plot) as Arc<dyn Plot>)
            />
            <p class="table-link">
                <a href="#videos">"Ver como tabela"</a>
            </p>
        </figure>

        <figure class="card">
            <figcaption>
                <h2 id="period-title">
                    {move || format!("Duração média por {}", if period.get() == Period::Year { "ano" } else { "mês" })}
                </h2>
                <p>"Média dos vídeos publicados em cada período, sem acumular os anteriores."</p>
            </figcaption>
            <Chart
                label="Gráfico da duração média por período"
                paint=move |size| groups.with(|groups| PeriodsPlot::new(groups.clone(), size)).map(|plot| Arc::new(plot) as Arc<dyn Plot>)
            />
            <details>
                <summary>"Ver como tabela"</summary>
                <div class="table-scroll">
                    <table>
                        <thead>
                            <tr>
                                <th scope="col">"Período"</th>
                                <th scope="col" class="num">"Vídeos"</th>
                                <th scope="col" class="num">"Duração média"</th>
                                <th scope="col" class="num">"Tempo total"</th>
                            </tr>
                        </thead>
                        <tbody id="period-rows">
                            {move || {
                                groups
                                    .get()
                                    .into_iter()
                                    .rev()
                                    .map(|group| {
                                        view! {
                                            <tr>
                                                <td>{group.label}</td>
                                                <td class="num">{format_count(group.count as u64)}</td>
                                                <td class="num">{group.mean.map_or("—".to_string(), format_duration)}</td>
                                                <td class="num">
                                                    {if group.count > 0 { format_total(group.total) } else { "—".to_string() }}
                                                </td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </tbody>
                    </table>
                </div>
            </details>
        </figure>

        <VideoTable videos=selected />
    }
}

/// A switch that adds a kind of video to the average.
#[component]
fn Toggle(id: &'static str, label: &'static str, on: RwSignal<bool>) -> impl IntoView {
    view! {
        <label>
            <input type="checkbox" id=id prop:checked=move || on.get() on:change=move |event| on.set(event_target_checked(&event)) />
            <span>{label}</span>
        </label>
    }
}

/// The address of the page without what comes after its directory.
fn site_url() -> String {
    let href = window().location().href().unwrap_or_default();
    web_sys::Url::new_with_base(".", &href)
        .map(|url| url.href())
        .unwrap_or(href)
}

fn link(url: String, text: String) -> impl IntoView {
    view! {
        <a href=url target="_blank" rel="noopener">
            {text}
        </a>
    }
}

/// How `videos` are grouped: by month when they span a short time, by year otherwise.
fn period_of(videos: &[RatedVideo]) -> Period {
    let span = match (videos.first(), videos.last()) {
        (Some(first), Some(last)) => {
            millis(last.video.published_at) - millis(first.video.published_at)
        }
        _ => 0.0,
    };
    if span < MONTHLY_SPAN {
        Period::Month
    } else {
        Period::Year
    }
}

/// Which way a variation went, as a class of the page, and its arrow.
fn direction(variation: f64) -> (&'static str, &'static str) {
    if variation.abs() < 0.005 {
        ("flat", "")
    } else if variation > 0.0 {
        ("up", "▲ ")
    } else {
        ("down", "▼ ")
    }
}

/// The rate as the headline shows it: "25min 13s", with the milliseconds apart, "450ms".
fn split_rate(mean: f64) -> (String, String) {
    let text = format_duration_ms(mean);
    match text.rsplit_once(' ') {
        Some((rate, milliseconds)) => (rate.to_string(), milliseconds.to_string()),
        None => (text, String::new()),
    }
}

/// "▲ +0,30s (+0,02%)", colored by its direction.
fn variation(video: &RatedVideo) -> impl IntoView + use<> {
    let (direction, arrow) = direction(video.variation);
    let text = format!(
        "{arrow}{} ({})",
        format_variation(video.variation),
        format_percent(video.variation_percent)
    );
    view! { <span class=format!("variation {direction}")>{text}</span> }
}

/// The headline and the tiles, for the videos that count.
#[component]
fn Summary<F>(counted: Memo<Vec<RatedVideo>>, open_tweet: F) -> impl IntoView
where
    F: Fn(leptos::ev::MouseEvent) + Copy + Send + Sync + 'static,
{
    move || {
        counted.with(|counted| {
            let summary = summarize(counted);
            let latest = counted.last();
            let (rate, milliseconds) = match latest {
                Some(latest) => split_rate(latest.mean),
                None => ("—".to_string(), String::new()),
            };
            let delta = latest.map(|latest| {
                view! {
                    {variation(latest)}
                    " "
                    <span class="context">"com o último vídeo"</span>
                }
            });
            let last_video = match latest {
                Some(latest) => view! {
                    "Último vídeo: "
                    {link(latest.video.url(), latest.video.title.clone())}
                    {format!(" · {} · {}", format_duration(latest.video.duration as f64), format_date(latest.video.published_at))}
                }
                .into_any(),
                None => "Nenhum vídeo entra na cotação com este filtro.".into_any(),
            };
            let tile = |value: Option<String>| value.unwrap_or_else(|| "—".to_string());
            let first: Option<&Video> = counted.first().map(|first| &first.video);
            view! {
                <section class="card hero" aria-labelledby="hero-label">
                    <p id="hero-label" class="label">
                        "1 Pirulla vale hoje"
                    </p>
                    <p class="hero-value">
                        <span id="rate">{rate}</span>
                        " "
                        <span id="rate-ms" class="ms">
                            {milliseconds}
                        </span>
                    </p>
                    <p id="delta" class="hero-delta">
                        {delta}
                    </p>
                    <p id="last-video" class="hero-note">
                        {last_video}
                    </p>
                    <p class="hero-actions">
                        <button id="tweet-open" class="button" type="button" hidden=latest.is_none() on:click=open_tweet>
                            "Gerar tweet"
                        </button>
                    </p>
                </section>

                <section class="tiles" aria-label="Números do canal">
                    <div class="card tile">
                        <p class="label">"Vídeos na cotação"</p>
                        <p id="tile-count" class="tile-value">
                            {tile(latest.map(|_| format_count(summary.count as u64)))}
                        </p>
                        <p id="tile-count-note" class="tile-note">
                            {first.map(|first| format!("desde {}", format_date(first.published_at)))}
                        </p>
                    </div>
                    <div class="card tile">
                        <p class="label">"Tempo total publicado"</p>
                        <p id="tile-total" class="tile-value">
                            {tile(latest.map(|_| format_total(summary.total)))}
                        </p>
                        <p id="tile-total-note" class="tile-note">
                            {latest
                                .map(|_| format!("{} horas de vídeo", format_count((summary.total as f64 / 3600.0).round() as u64)))}
                        </p>
                    </div>
                    <div class="card tile">
                        <p class="label">"Vídeo mais longo"</p>
                        <p id="tile-longest" class="tile-value">
                            {tile(summary.longest.map(|longest| format_duration(longest.video.duration as f64)))}
                        </p>
                        <p id="tile-longest-note" class="tile-note">
                            {summary.longest.map(|longest| link(longest.video.url(), longest.video.title.clone()))}
                        </p>
                    </div>
                </section>
            }
        })
    }
}

/// The title of a video, linked, with a mark when it is a live or a short.
fn title(video: &Video) -> impl IntoView + use<> {
    let kind = (video.kind != VideoKind::Video)
        .then(|| view! { <span class="tag">{format_kind(video.kind)}</span> });
    view! {
        {link(video.url(), video.title.clone())}
        {kind}
    }
}

/// The table of videos, most recent first, shown a page at a time.
#[component]
fn VideoTable(videos: Memo<Vec<RatedVideo>>) -> impl IntoView {
    let shown = RwSignal::new(FIRST_ROWS);
    // Other videos start again from the first page.
    Effect::watch(
        move || videos.track(),
        move |_, _, _| shown.set(FIRST_ROWS),
        false,
    );
    let remaining = move || videos.with(|videos| videos.len().saturating_sub(shown.get()));

    view! {
        <section id="videos" class="card" aria-labelledby="videos-title">
            <h2 id="videos-title">"Vídeos e o efeito de cada um na cotação"</h2>
            <div class="table-scroll">
                <table class="stacks">
                    <thead>
                        <tr>
                            <th scope="col">"Data"</th>
                            <th scope="col">"Vídeo"</th>
                            <th scope="col" class="num">"Duração"</th>
                            <th scope="col" class="num">"Cotação"</th>
                            <th scope="col" class="num">"Variação"</th>
                        </tr>
                    </thead>
                    <tbody id="video-rows">
                        {move || {
                            videos
                                .with(|videos| {
                                    if videos.is_empty() {
                                        return view! {
                                            <tr>
                                                <td class="empty" colspan="5">
                                                    "Sem vídeos neste período."
                                                </td>
                                            </tr>
                                        }
                                            .into_any();
                                    }
                                    videos
                                        .iter()
                                        .rev()
                                        .take(shown.get())
                                        .map(|video| {
                                            view! {
                                                <tr>
                                                    <td class="date">{format_date(video.video.published_at)}</td>
                                                    <td class="title">{title(&video.video)}</td>
                                                    <td class="num" data-label="Duração">
                                                        {format_duration(video.video.duration as f64)}
                                                    </td>
                                                    <td class="num" data-label="Cotação">
                                                        {format_duration_ms(video.mean)}
                                                    </td>
                                                    <td class="num">{variation(video)}</td>
                                                </tr>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                })
                        }}
                    </tbody>
                </table>
            </div>
            <button
                class="button more"
                type="button"
                hidden=move || remaining() == 0
                on:click=move |_| shown.update(|shown| *shown += MORE_ROWS)
            >
                {move || format!("Mostrar mais ({} restantes)", format_count(remaining() as u64))}
            </button>
        </section>
    }
}

/// The dialog with the post announcing the latest video, which can be edited,
/// copied or opened in X.
#[component]
fn TweetDialog(dialog: NodeRef<html::Dialog>, text: RwSignal<String>) -> impl IntoView {
    let textarea = NodeRef::<html::Textarea>::new();
    let copy_label = RwSignal::new("Copiar texto");
    let length = move || text.with(|text| weighted_length(text));
    // Writing a new text, or opening the dialog with one, takes back what the copy button said.
    Effect::watch(
        move || text.track(),
        move |_, _, _| copy_label.set("Copiar texto"),
        false,
    );

    let copy = move |_| {
        let value = text.get_untracked();
        spawn_local(async move {
            let copied = JsFuture::from(window().navigator().clipboard().write_text(&value)).await;
            match copied {
                Ok(_) => copy_label.set("Copiado!"),
                Err(_) => {
                    // Without access to the clipboard, the text is left selected for the reader to copy.
                    if let Some(textarea) = textarea.get_untracked() {
                        textarea.select();
                    }
                    copy_label.set("Selecionado: copie com Ctrl+C");
                }
            }
        });
    };

    view! {
        <dialog
            id="tweet-dialog"
            aria-labelledby="tweet-title"
            node_ref=dialog
            // A click on the backdrop lands on the dialog itself.
            on:click=move |event| {
                if let Some(dialog) = dialog.get_untracked()
                    && event.target().as_ref() == Some::<&web_sys::EventTarget>(dialog.as_ref())
                {
                    dialog.close();
                }
            }
        >
            <form method="dialog" class="dialog-body">
                <h2 id="tweet-title">"Tweet do último vídeo"</h2>
                <p class="dialog-note">"Ajuste o texto se quiser, depois copie ou abra o X com ele já escrito."</p>
                <textarea
                    id="tweet-text"
                    node_ref=textarea
                    rows="9"
                    spellcheck="false"
                    aria-label="Texto do tweet"
                    aria-describedby="tweet-count"
                    prop:value=move || text.get()
                    on:input=move |event| text.set(event_target_value(&event))
                ></textarea>
                <p id="tweet-count" class="dialog-note" class:over={move || length() > MAX_TWEET_LENGTH} role="status">
                    {move || {
                        let length = length();
                        if length > MAX_TWEET_LENGTH {
                            format!("{length} de {MAX_TWEET_LENGTH} caracteres: o X não aceita um texto deste tamanho.")
                        } else {
                            format!("{length} de {MAX_TWEET_LENGTH} caracteres, na contagem do X.")
                        }
                    }}
                </p>
                <div class="dialog-actions">
                    <button class="button quiet">"Fechar"</button>
                    <button id="tweet-copy" class="button" type="button" autofocus on:click=copy>
                        {move || copy_label.get()}
                    </button>
                    <a
                        id="tweet-post"
                        class="button primary"
                        target="_blank"
                        rel="noopener"
                        href=move || text.with(|text| intent_url(text))
                    >
                        "Postar no X"
                    </a>
                </div>
            </form>
        </dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rated(published_at: &str) -> RatedVideo {
        RatedVideo {
            video: Video {
                id: "a".to_string(),
                title: "A".to_string(),
                published_at: published_at.parse().unwrap(),
                duration: 600,
                kind: VideoKind::Video,
            },
            mean: 600.0,
            variation: 0.0,
            variation_percent: 0.0,
        }
    }

    #[test]
    fn the_headline_shows_the_milliseconds_apart() {
        assert_eq!(
            split_rate(1513.45),
            ("25min 13s".to_string(), "450ms".to_string())
        );
        assert_eq!(split_rate(45.0), ("45s".to_string(), "000ms".to_string()));
        assert_eq!(
            split_rate(3723.5),
            ("1h 02min 03s".to_string(), "500ms".to_string())
        );
    }

    #[test]
    fn a_variation_goes_up_down_or_nowhere() {
        assert_eq!(direction(0.3), ("up", "▲ "));
        assert_eq!(direction(-2.48), ("down", "▼ "));
        assert_eq!(direction(0.004), ("flat", ""));
        assert_eq!(direction(-0.004), ("flat", ""));
    }

    #[test]
    fn videos_over_a_short_time_are_grouped_by_month() {
        let videos = [rated("2024-01-01T12:00:00Z"), rated("2026-06-01T12:00:00Z")];

        assert_eq!(period_of(&videos), Period::Month);
        assert_eq!(period_of(&videos[..1]), Period::Month);
        assert_eq!(period_of(&[]), Period::Month);
    }

    #[test]
    fn videos_over_a_long_time_are_grouped_by_year() {
        let videos = [rated("2024-01-01T12:00:00Z"), rated("2026-08-01T12:00:00Z")];

        assert_eq!(period_of(&videos), Period::Year);
    }
}
