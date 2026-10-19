//! Runs the built website in Chrome, as a reader would use it.
//!
//! The website must be built first, with `trunk build --release` in `site/`.
//! Chrome is found on the `PATH`, or at the path in the `CHROME` environment variable.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use headless_chrome::browser::tab::ModifierKey;
use headless_chrome::browser::tab::point::Point;
use headless_chrome::{Browser, LaunchOptions, Tab};
use pirulla_core::tweet::{intent_url, weighted_length};
use serde_json::{Value, json};

/// Where GitHub Pages serves the website from.
const DIRECTORY: &str = "/pirulla-bot/";
const PATIENCE: Duration = Duration::from_secs(10);

/// The website as `trunk build --release` left it.
fn dist() -> PathBuf {
    let site = Path::new(env!("CARGO_MANIFEST_DIR"));
    let built = site.join("dist");
    let built_at = fs::metadata(built.join("index.html"))
        .and_then(|metadata| metadata.modified())
        .unwrap_or_else(|_| {
            panic!(
                "build the website first: `trunk build --release` in {}",
                site.display()
            )
        });
    let newest_source = [
        site.join("src"),
        site.join("../core/src"),
        site.join("index.html"),
        site.join("style.css"),
    ]
    .iter()
    .map(|path| newest(path))
    .max()
    .unwrap();
    assert!(
        built_at >= newest_source,
        "the website in {} is older than its sources: build it again with `trunk build --release`",
        built.display()
    );
    built
}

/// When the newest file under `path` was changed.
fn newest(path: &Path) -> SystemTime {
    let changed = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap();
    if !path.is_dir() {
        return changed;
    }
    fs::read_dir(path)
        .unwrap()
        .map(|entry| newest(&entry.unwrap().path()))
        .fold(changed, SystemTime::max)
}

/// Serves the built website, with `data` as its data file, or no data file at all.
/// Answers the address of the website.
fn serve(data: Option<String>) -> String {
    let dist = dist();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (dist, data) = (dist.clone(), data.clone());
            thread::spawn(move || answer(stream, &dist, data.as_deref()));
        }
    });
    format!("http://{address}{DIRECTORY}")
}

fn answer(mut stream: TcpStream, dist: &Path, data: Option<&str>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request = String::new();
    reader.read_line(&mut request).unwrap();
    // The rest of the request does not matter, but it has to be read.
    let mut line = String::new();
    while reader.read_line(&mut line).unwrap_or(0) > 2 {
        line.clear();
    }
    let path = request.split_whitespace().nth(1).unwrap_or("/");
    let file = path
        .split('?')
        .next()
        .unwrap()
        .strip_prefix(DIRECTORY)
        .map(|file| match file {
            "" => "index.html",
            file => file,
        });
    let body = match file {
        Some("data.json") => data.map(|data| data.as_bytes().to_vec()),
        Some(file) if !file.contains("..") => fs::read(dist.join(file)).ok(),
        _ => None,
    };
    let kind = match file
        .and_then(|file| file.rsplit_once('.'))
        .map(|(_, extension)| extension)
    {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript",
        Some("wasm") => "application/wasm",
        Some("css") => "text/css",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    };
    let (status, body) = match body {
        Some(body) => ("200 OK", body),
        None => ("404 Not Found", b"not found".to_vec()),
    };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.write_all(&body));
}

/// A Chrome of its own for each test, closed when the test ends.
fn launch() -> Browser {
    let options = LaunchOptions::default_builder()
        // The runners of GitHub cannot give Chrome its sandbox; the tests only open the website.
        .sandbox(false)
        .window_size(Some((1280, 900)))
        .idle_browser_timeout(Duration::from_secs(120))
        .build()
        .unwrap();
    Browser::new(options)
        .expect("could not start Chrome: put it on the PATH, or its path in CHROME")
}

/// The website open in a tab of its own.
struct Page {
    tab: Arc<Tab>,
    url: String,
    // Last, so that Chrome closes after its tab.
    _browser: Browser,
}

impl Page {
    fn open(data: Option<String>) -> Self {
        let url = serve(data);
        let browser = launch();
        let tab = browser.new_tab().unwrap();
        tab.set_default_timeout(PATIENCE);
        tab.navigate_to(&url)
            .unwrap()
            .wait_until_navigated()
            .unwrap();
        // The page is drawn once the WebAssembly and the data are in.
        tab.wait_for_element(".hero-value, #status").unwrap();
        Self {
            tab,
            url,
            _browser: browser,
        }
    }

    /// The value of the JavaScript expression `expression`, read from the page.
    fn read(&self, expression: &str) -> Value {
        let script = format!("JSON.stringify((() => {{ return {expression}; }})())");
        let result = self.tab.evaluate(&script, false).unwrap();
        match result.value {
            Some(Value::String(json)) => serde_json::from_str(&json).unwrap(),
            _ => panic!("could not read {expression}: {:?}", result.description),
        }
    }

    fn run(&self, statement: &str) {
        self.tab
            .evaluate(&format!("{{ {statement} }}"), false)
            .unwrap();
    }

    /// The text of the first element that matches `selector`, on one line.
    fn text(&self, selector: &str) -> String {
        self.read(&format!(
            "document.querySelector({selector:?})?.textContent.replace(/\\s+/g, ' ').trim() ?? null"
        ))
        .as_str()
        .unwrap_or_else(|| panic!("nothing matches {selector}"))
        .to_string()
    }

    /// The text of every element that matches `selector`, each on one line.
    fn texts(&self, selector: &str) -> Vec<String> {
        serde_json::from_value(self.read(&format!(
            "[...document.querySelectorAll({selector:?})].map((e) => e.textContent.replace(/\\s+/g, ' ').trim())"
        )))
        .unwrap()
    }

    /// The text of every element that matches `selector` as the reader sees it, with its
    /// cells and blocks apart, each on one line.
    fn shown(&self, selector: &str) -> Vec<String> {
        serde_json::from_value(self.read(&format!(
            "[...document.querySelectorAll({selector:?})].map((e) => e.innerText.replace(/\\s+/g, ' ').trim())"
        )))
        .unwrap()
    }

    fn is_hidden(&self, selector: &str) -> bool {
        self.read(&format!("document.querySelector({selector:?}).hidden"))
            .as_bool()
            .unwrap()
    }

    /// Waits until `look` sees `expected`, and fails with what it last saw if it never does.
    fn eventually<T: PartialEq + std::fmt::Debug>(&self, expected: T, look: impl Fn(&Self) -> T) {
        let started = Instant::now();
        loop {
            let seen = look(self);
            if seen == expected {
                return;
            }
            if started.elapsed() > PATIENCE {
                assert_eq!(seen, expected);
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn click(&self, selector: &str) {
        self.tab.find_element(selector).unwrap().click().unwrap();
    }

    /// Clicks the switch or the option labelled `label`.
    fn choose(&self, label: &str) {
        let option = format!(
            "//div[contains(concat(' ', @class, ' '), ' options ')]/label[span = '{label}']"
        );
        self.tab
            .find_element_by_xpath(&option)
            .unwrap()
            .click()
            .unwrap();
    }

    /// What the tooltip shown, if any, says.
    fn tooltip(&self) -> Option<String> {
        self.read(
            "[...document.querySelectorAll('.tooltip')].filter((t) => !t.hidden).map((t) => [...t.children].map((c) => c.textContent.replace(/\\s+/g, ' ').trim()))[0] ?? null",
        )
        .as_array()
        .map(|lines| lines.iter().map(|line| line.as_str().unwrap().to_string()).collect::<Vec<_>>().join(" | "))
    }

    /// Puts the keyboard on the chart `index` (0 for the rate, 1 for the videos, 2 for the periods).
    fn focus_chart(&self, index: usize) {
        self.run(&format!(
            "document.querySelectorAll('.chart')[{index}].focus()"
        ));
    }

    fn press(&self, key: &str) {
        self.tab.press_key(key).unwrap();
    }

    /// The point of the page at `x`, `y` of the chart `index`, which is scrolled into view.
    fn chart_point(&self, index: usize, x: f64, y: f64) -> Point {
        self.run(&format!(
            "document.querySelectorAll('.chart')[{index}].scrollIntoView({{block: 'center', behavior: 'instant'}})"
        ));
        let corner = self.read(&format!(
            "(() => {{ const box = document.querySelectorAll('.chart')[{index}].getBoundingClientRect(); return [box.left, box.top]; }})()"
        ));
        Point {
            x: corner[0].as_f64().unwrap() + x,
            y: corner[1].as_f64().unwrap() + y,
        }
    }

    /// Keeps the addresses the page opens instead of opening them.
    fn catch_opened(&self) {
        self.run(
            "window.opened = []; window.open = (url) => { window.opened.push(url); return null; };",
        );
    }

    fn opened(&self) -> Vec<String> {
        serde_json::from_value(self.read("window.opened")).unwrap()
    }
}

fn video(id: &str, title: &str, published_at: &str, duration: u64, kind: &str) -> Value {
    json!({"id": id, "title": title, "publishedAt": published_at, "duration": duration, "kind": kind})
}

fn data(videos: Vec<Value>) -> String {
    json!({"generatedAt": "2026-10-13T19:24:58Z", "channelId": "UCfixture", "videos": videos})
        .to_string()
}

/// Three regular videos, a live and a short. By default the live is left out, and the
/// average goes 10min, 15min, 10min 20s and 11min 30s.
fn channel() -> String {
    data(vec![
        video("a", "Primeiro vídeo", "2020-01-10T15:00:00Z", 600, "video"),
        video("b", "Uma live longa", "2021-06-05T23:00:00Z", 7200, "live"),
        video("c", "Segundo vídeo", "2022-03-01T15:00:00Z", 1200, "video"),
        video("d", "Um short", "2023-05-20T15:00:00Z", 60, "short"),
        video("e", "Terceiro vídeo", "2026-10-11T18:00:00Z", 900, "video"),
    ])
}

fn headline(page: &Page) -> (String, String, String) {
    (
        page.text("#rate"),
        page.text("#rate-ms"),
        page.text("#delta"),
    )
}

#[test]
fn shows_the_value_of_1_pirulla_and_the_numbers_of_the_channel() {
    let page = Page::open(Some(channel()));

    assert_eq!(
        headline(&page),
        (
            "11min 30s".into(),
            "000ms".into(),
            "▲ +1min 10s (+11,29%) com o último vídeo".into()
        )
    );
    assert_eq!(
        page.read("document.querySelector('#delta .variation').className"),
        "variation up"
    );
    assert_eq!(
        page.text("#last-video"),
        "Último vídeo: Terceiro vídeo · 15min 00s · 11/10/2026"
    );
    assert_eq!(
        page.read("document.querySelector('#last-video a').href"),
        "https://www.youtube.com/watch?v=e"
    );
    assert_eq!(
        page.shown(".tile"),
        [
            "Vídeos na cotação 4 desde 10/01/2020",
            "Tempo total publicado 0h 46min 1 horas de vídeo",
            "Vídeo mais longo 20min 00s Segundo vídeo",
        ]
    );
    assert_eq!(page.text("#updated"), "Atualizado em 13/10/2026 16:24.");
    assert_eq!(
        page.read("document.querySelector('#channel-link').href"),
        "https://www.youtube.com/channel/UCfixture"
    );
    assert_eq!(page.read("document.title"), "Cotação do Pirulla");
}

#[test]
fn the_switches_choose_what_counts_in_the_average() {
    let page = Page::open(Some(channel()));
    let checked = |page: &Page| {
        page.read("[...document.querySelectorAll('.toggles input')].map((i) => i.checked)")
    };
    assert_eq!(checked(&page), json!([false, true]));

    page.choose("Lives");

    page.eventually(
        (
            "33min 12s".into(),
            "000ms".into(),
            "▼ −4min 33s (−12,05%) com o último vídeo".into(),
        ),
        headline,
    );
    assert_eq!(
        page.shown(".tile"),
        [
            "Vídeos na cotação 5 desde 10/01/2020",
            "Tempo total publicado 2h 46min 3 horas de vídeo",
            "Vídeo mais longo 2h 00min 00s Uma live longa",
        ]
    );

    page.choose("Lives");
    page.choose("Shorts");

    // 10min, 20min and 15min: the last video left the average where it was.
    page.eventually(
        (
            "15min 00s".into(),
            "000ms".into(),
            "0,00s (0,00%) com o último vídeo".into(),
        ),
        headline,
    );
    assert_eq!(
        page.read("document.querySelector('#delta .variation').className"),
        "variation flat"
    );
    assert_eq!(checked(&page), json!([false, false]));
}

#[test]
fn the_period_scopes_the_charts_and_the_tables() {
    let page = Page::open(Some(channel()));
    let period_rows = |page: &Page| page.shown("#period-rows tr");
    // The table of the periods is folded under the chart.
    assert_eq!(period_rows(&page), [""; 7]);
    page.click("details summary");

    assert_eq!(page.text("#period-title"), "Duração média por ano");
    assert_eq!(
        period_rows(&page),
        [
            "2026 1 15min 00s 0h 15min",
            "2025 0 — —",
            "2024 0 — —",
            "2023 1 1min 00s 0h 01min",
            "2022 1 20min 00s 0h 20min",
            "2021 0 — —",
            "2020 1 10min 00s 0h 10min",
        ]
    );
    assert_eq!(page.texts("#video-rows tr").len(), 4);

    page.choose("1 ano");

    page.eventually(vec!["out/26 1 15min 00s 0h 15min".to_string()], period_rows);
    assert_eq!(page.text("#period-title"), "Duração média por mês");
    assert_eq!(
        page.shown("#video-rows tr"),
        ["11/10/2026 Terceiro vídeo 15min 00s 11min 30s 000ms ▲ +1min 10s (+11,29%)"]
    );
    // The headline is about the whole channel, whatever the period.
    assert_eq!(page.text("#rate"), "11min 30s");
}

#[test]
fn the_table_shows_each_video_and_how_it_moved_the_average() {
    let page = Page::open(Some(channel()));

    let rows: Vec<Vec<String>> = serde_json::from_value(page.read(
        "[...document.querySelectorAll('#video-rows tr')].map((r) => [...r.children].map((c) => c.textContent.trim()))",
    ))
    .unwrap();

    assert_eq!(
        rows,
        [
            [
                "11/10/2026",
                "Terceiro vídeo",
                "15min 00s",
                "11min 30s 000ms",
                "▲ +1min 10s (+11,29%)"
            ],
            [
                "20/05/2023",
                "Um shortshort",
                "1min 00s",
                "10min 20s 000ms",
                "▼ −4min 40s (−31,11%)"
            ],
            [
                "01/03/2022",
                "Segundo vídeo",
                "20min 00s",
                "15min 00s 000ms",
                "▲ +5min 00s (+50,00%)"
            ],
            [
                "10/01/2020",
                "Primeiro vídeo",
                "10min 00s",
                "10min 00s 000ms",
                "0,00s (0,00%)"
            ],
        ]
    );
    assert_eq!(page.texts("#video-rows .tag"), ["short"]);
    assert_eq!(
        page.read("[...document.querySelectorAll('#video-rows td[data-label]')].slice(0, 2).map((c) => c.dataset.label)"),
        json!(["Duração", "Cotação"])
    );
    assert!(page.is_hidden("button.more"));
}

#[test]
fn the_table_shows_the_videos_a_page_at_a_time() {
    // 40 videos, 15 days apart, the last one in August 2026.
    let start = chrono::DateTime::parse_from_rfc3339("2025-01-01T15:00:00Z").unwrap();
    let videos = (0..40)
        .map(|index| {
            let published_at = (start + chrono::Duration::days(15 * index)).to_rfc3339();
            video(
                &format!("v{index}"),
                &format!("Vídeo {index}"),
                &published_at,
                600,
                "video",
            )
        })
        .collect();
    let page = Page::open(Some(data(videos)));
    let table = |page: &Page| {
        (
            page.texts("#video-rows tr").len(),
            page.text("button.more"),
            page.is_hidden("button.more"),
        )
    };

    assert_eq!(
        table(&page),
        (15, "Mostrar mais (25 restantes)".to_string(), false)
    );

    page.click("button.more");

    page.eventually((40, "Mostrar mais (0 restantes)".to_string(), true), table);

    // Another period starts again from the first page: 21 videos came out in the last year.
    page.choose("1 ano");

    page.eventually((15, "Mostrar mais (6 restantes)".to_string(), false), table);
}

#[test]
fn without_videos_that_count_the_page_says_so() {
    let page = Page::open(Some(data(vec![
        video("l", "Uma live", "2026-10-01T23:00:00Z", 5400, "live"),
        video("s", "Um short", "2026-10-05T15:00:00Z", 45, "short"),
    ])));

    page.choose("Shorts");

    page.eventually(("—".into(), "".into(), "".into()), headline);
    assert_eq!(
        page.text("#last-video"),
        "Nenhum vídeo entra na cotação com este filtro."
    );
    assert_eq!(page.texts(".tile-value"), ["—", "—", "—"]);
    assert_eq!(page.texts(".tile-note"), ["", "", ""]);
    assert!(page.is_hidden("#tweet-open"));
    assert_eq!(page.texts(".chart-empty"), ["Sem vídeos neste período."; 3]);
    assert_eq!(page.texts("#video-rows tr"), ["Sem vídeos neste período."]);
    assert!(page.is_hidden("button.more"));
}

#[test]
fn the_charts_show_the_values_under_the_pointer() {
    let page = Page::open(Some(channel()));
    page.catch_opened();
    assert_eq!(
        page.read("document.querySelectorAll('.chart svg').length"),
        3
    );

    // The last dot of the durations, the most recent video.
    let dot = page.read(
        "(() => { const c = document.querySelectorAll('.chart')[1].querySelector('.dots circle:last-child'); return [+c.getAttribute('cx'), +c.getAttribute('cy')]; })()",
    );
    let point = page.chart_point(1, dot[0].as_f64().unwrap(), dot[1].as_f64().unwrap());
    page.tab.move_mouse_to_point(point).unwrap();

    page.eventually(
        Some(
            "Terceiro vídeo | 15min 00s Duração | 11min 30s Cotação na data | 11/10/2026"
                .to_string(),
        ),
        Page::tooltip,
    );
    assert_eq!(
        page.read("getComputedStyle(document.querySelectorAll('.chart svg')[1]).cursor"),
        "pointer"
    );
    // The tooltip stays inside the chart, to the left of the dot, since there is no room to its right.
    page.eventually(json!([true, true]), |page| {
        page.read(&format!(
            "(() => {{ const c = document.querySelectorAll('.chart')[1].getBoundingClientRect(), t = document.querySelectorAll('.tooltip')[1].getBoundingClientRect(); return [t.left >= c.left && t.right <= c.right && t.top >= c.top && t.bottom <= c.bottom, t.right <= c.left + {}]; }})()",
            dot[0].as_f64().unwrap()
        ))
    });

    page.tab.click_point(point).unwrap();

    page.eventually(
        vec!["https://www.youtube.com/watch?v=e".to_string()],
        Page::opened,
    );

    // A pointer down anywhere else puts the tooltip away.
    page.click("h1");

    page.eventually(None, Page::tooltip);

    // On the rate, the pointer selects the video nearest in time.
    let width = page
        .read("document.querySelectorAll('.chart')[0].clientWidth")
        .as_f64()
        .unwrap();
    page.tab
        .move_mouse_to_point(page.chart_point(0, width - 120.0, 150.0))
        .unwrap();

    page.eventually(
        Some("11/10/2026 | 11min 30s 000ms Cotação | +1min 10s (+11,29%) com este vídeo | Terceiro vídeo · 15min 00s".to_string()),
        Page::tooltip,
    );
}

#[test]
fn the_charts_can_be_read_with_the_keyboard() {
    let page = Page::open(Some(channel()));
    page.catch_opened();
    let title = |page: &Page| {
        page.tooltip()
            .map(|tooltip| tooltip.split(" | ").next().unwrap().to_string())
    };

    page.focus_chart(0);
    page.press("End");
    page.eventually(Some("11/10/2026".to_string()), title);
    page.press("Home");
    page.eventually(Some("10/01/2020".to_string()), title);
    page.press("ArrowRight");
    page.eventually(Some("01/03/2022".to_string()), title);
    page.tab
        .press_key_with_modifiers("ArrowRight", Some(&[ModifierKey::Shift]))
        .unwrap();
    // Ten steps at a time, but never past the last video.
    page.eventually(Some("11/10/2026".to_string()), title);
    page.press("Escape");
    page.eventually(None, Page::tooltip);

    page.focus_chart(2);
    page.press("End");
    page.eventually(
        Some("2026 | 15min 00s Duração média | 1 vídeo | 0h 15min no total".to_string()),
        Page::tooltip,
    );
    page.press("ArrowLeft");
    page.eventually(
        Some("2025 | Nenhum vídeo publicado".to_string()),
        Page::tooltip,
    );

    // Leaving the chart puts the tooltip away.
    page.run("document.querySelector('#tweet-open').focus()");
    page.eventually(None, Page::tooltip);

    // Enter opens the video under the keyboard.
    page.focus_chart(1);
    page.press("Home");
    page.eventually(Some("Primeiro vídeo".to_string()), title);
    page.press("Enter");
    page.eventually(
        vec!["https://www.youtube.com/watch?v=a".to_string()],
        Page::opened,
    );
}

#[test]
fn the_charts_follow_the_width_of_the_page() {
    let page = Page::open(Some(channel()));
    // The width of each chart, its height and how many values are written on it.
    let charts = |page: &Page| -> Vec<(f64, f64, usize)> {
        serde_json::from_value(page.read(
            "[...document.querySelectorAll('.chart')].map((c) => { const s = c.querySelector('svg'); return [+s.getAttribute('width'), +s.getAttribute('height'), s.querySelectorAll('.direct-label').length]; })",
        ))
        .unwrap()
    };
    let width = |page: &Page| {
        page.read("document.querySelector('.chart').clientWidth")
            .as_f64()
            .unwrap()
    };
    let wide = width(&page);
    assert!(wide >= 520.0, "{wide}");
    // The periods label the highest column, 2022, and the latest, 2026.
    assert_eq!(
        charts(&page),
        [(wide, 340.0, 1), (wide, 420.0, 1), (wide, 340.0, 2)]
    );

    // On a narrow page the charts are shorter, and the rate and the durations lose their labels.
    page.run("document.querySelector('main').style.maxWidth = '480px'");

    page.eventually(true, |page| width(page) < 520.0);
    let narrow = width(&page);
    page.eventually(
        vec![(narrow, 279.0, 0), (narrow, 344.0, 0), (narrow, 279.0, 2)],
        charts,
    );

    page.run("document.querySelector('main').style.maxWidth = ''");

    page.eventually(
        vec![(wide, 340.0, 1), (wide, 420.0, 1), (wide, 340.0, 2)],
        charts,
    );
}

#[test]
fn the_post_about_the_latest_video_can_be_edited_copied_and_opened_in_x() {
    let page = Page::open(Some(channel()));
    let post = format!(
        "🚨 VÍDEO NOVO 🚨\n🎥 Terceiro vídeo\n🕒 Duração: 15min 00s\n🔗 https://www.youtube.com/watch?v=e\n\n⏳ 1 Pirulla: 11min 30s 000ms\n📊 Variação: 📈 +1min 10s (+11,29%)\n🌐 Gráficos: {}",
        page.url
    );
    let dialog = |page: &Page| -> (bool, String, String, String) {
        serde_json::from_value(page.read(
            "[document.querySelector('dialog').open, document.querySelector('#tweet-text').value, document.querySelector('#tweet-count').textContent, document.querySelector('#tweet-post').href]",
        ))
        .unwrap()
    };
    let length = weighted_length(&post);

    page.click("#tweet-open");

    page.eventually(
        (
            true,
            post.clone(),
            format!("{length} de 280 caracteres, na contagem do X."),
            intent_url(&post),
        ),
        dialog,
    );
    assert_eq!(page.text("#tweet-copy"), "Copiar texto");

    // Copied, when the browser lets it.
    page.run("navigator.clipboard.writeText = (text) => { window.copied = text; return Promise.resolve(); }");
    page.click("#tweet-copy");
    page.eventually("Copiado!".to_string(), |page| page.text("#tweet-copy"));
    assert_eq!(page.read("window.copied"), json!(post));

    // Edited by hand.
    page.run("const t = document.querySelector('#tweet-text'); t.focus(); t.setSelectionRange(t.value.length, t.value.length);");
    page.tab.type_str(" #extra").unwrap();
    let edited = format!("{post} #extra");
    page.eventually(
        (
            true,
            edited.clone(),
            format!("{} de 280 caracteres, na contagem do X.", length + 7),
            intent_url(&edited),
        ),
        dialog,
    );
    assert_eq!(page.text("#tweet-copy"), "Copiar texto");

    // Too long for X.
    page.run("const t = document.querySelector('#tweet-text'); t.value = 'x'.repeat(300); t.dispatchEvent(new Event('input', {bubbles: true}));");
    page.eventually(
        "300 de 280 caracteres: o X não aceita um texto deste tamanho.".to_string(),
        |page| page.text("#tweet-count"),
    );
    assert_eq!(
        page.read("document.querySelector('#tweet-count').classList.contains('over')"),
        true
    );

    // Selected for the reader to copy, when the browser does not let it.
    page.run("navigator.clipboard.writeText = () => Promise.reject(new Error('denied'))");
    page.click("#tweet-copy");
    page.eventually("Selecionado: copie com Ctrl+C".to_string(), |page| {
        page.text("#tweet-copy")
    });
    assert_eq!(
        page.read("(() => { const t = document.querySelector('#tweet-text'); return [t.selectionStart, t.selectionEnd]; })()"),
        json!([0, 300])
    );

    // A click on the backdrop closes it, and it opens again with the post as it was written.
    page.tab.click_point(Point { x: 5.0, y: 5.0 }).unwrap();
    page.eventually(false, |page| {
        page.read("document.querySelector('dialog').open")
            .as_bool()
            .unwrap()
    });
    page.click("#tweet-open");
    page.eventually(
        (
            true,
            post.clone(),
            format!("{length} de 280 caracteres, na contagem do X."),
            intent_url(&post),
        ),
        dialog,
    );
    assert_eq!(page.text("#tweet-copy"), "Copiar texto");

    page.click("dialog button.quiet");
    page.eventually(false, |page| {
        page.read("document.querySelector('dialog').open")
            .as_bool()
            .unwrap()
    });
}

#[test]
fn the_post_follows_the_switches() {
    let page = Page::open(Some(channel()));

    page.choose("Lives");
    page.eventually("33min 12s".to_string(), |page| page.text("#rate"));
    page.click("#tweet-open");

    page.eventually(Some("⏳ 1 Pirulla: 33min 12s 000ms".to_string()), |page| {
        page.read("document.querySelector('#tweet-text').value.split('\\n')[5] ?? null")
            .as_str()
            .map(String::from)
    });
    assert_eq!(
        page.read("document.querySelector('#tweet-text').value.split('\\n')[6]"),
        "📊 Variação: 📉 −4min 33s (−12,05%)"
    );
}

#[test]
fn a_failure_to_load_the_data_is_explained() {
    for data in [None, Some("{ not json".to_string()), Some(data(vec![]))] {
        let page = Page::open(data.clone());

        assert_eq!(
            page.text("#status"),
            "Não foi possível carregar os dados agora. Tente recarregar a página em alguns minutos.",
            "{data:?}"
        );
        assert_eq!(
            page.read("document.querySelectorAll('.hero, .chart').length"),
            0
        );
    }
}
