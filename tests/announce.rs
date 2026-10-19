//! Runs the compiled announcer as the workflow does, against a fake X.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use chrono::{Duration, Utc};
use mockito::{Matcher, Mock, Server, ServerGuard};
use serde_json::json;

const SITE_URL: &str = "https://example.github.io/pirulla-bot/";
const KEYS: [(&str, &str); 4] = [
    ("TWITTER_API_KEY", "consumer-key"),
    ("TWITTER_API_SECRET", "consumer-secret"),
    ("ACCESS_TOKEN", "access-token"),
    ("ACCESS_SECRET_TOKEN", "access-secret"),
];

/// Runs the announcer in `dir`, isolated from the environment of whoever runs the
/// tests, so that no real key is ever used.
fn run(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_announce"))
        .args(args)
        .current_dir(dir)
        .env_clear()
        .envs(env.iter().copied())
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn log(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A data file with a video for each of `videos`: its id, how many hours ago it came out and its kind.
fn data_file(dir: &Path, name: &str, videos: &[(&str, i64, &str)]) -> PathBuf {
    let videos: Vec<_> = videos
        .iter()
        .map(|(id, hours_ago, kind)| {
            json!({
                "id": id,
                "title": format!("Vídeo {id}"),
                "publishedAt": (Utc::now() - Duration::hours(*hours_ago)).to_rfc3339(),
                "duration": 600,
                "kind": kind,
            })
        })
        .collect();
    let path = dir.join(name);
    let data =
        json!({"generatedAt": Utc::now().to_rfc3339(), "channelId": "UCchannel", "videos": videos});
    fs::write(&path, data.to_string()).unwrap();
    path
}

fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// Makes the fake X take `text` as the post `id`, once.
fn takes(x: &mut ServerGuard, text: &str, id: &str) -> Mock {
    x.mock("POST", "/2/tweets")
        .match_header(
            "authorization",
            Matcher::Regex(
                r#"^OAuth oauth_consumer_key="consumer-key", .*oauth_token="access-token", "#
                    .to_string(),
            ),
        )
        .match_body(Matcher::Json(json!({"text": text})))
        .with_status(201)
        .with_body(json!({"data": {"id": id, "text": text}}).to_string())
        .expect(1)
        .create()
}

fn post(dir: &Path, x: &ServerGuard, posts: &[&str], keys: bool) -> Output {
    let posts = serde_json::to_string(posts).unwrap();
    let mut env = vec![("X_API_URL", x.url()), ("POSTS", posts)];
    if keys {
        env.extend(KEYS.map(|(name, value)| (name, value.to_string())));
    }
    let env: Vec<(&str, &str)> = env
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect();
    run(dir, &["post"], &env)
}

#[test]
fn compose_writes_the_posts_for_the_new_videos_as_a_json_list_on_one_line() {
    let dir = tempfile::tempdir().unwrap();
    let known = [("a", 500, "video"), ("b", 400, "video")];
    let previous = data_file(dir.path(), "previous.json", &known);
    let current = data_file(
        dir.path(),
        "current.json",
        &[known[0], known[1], ("live", 3, "live"), ("c", 2, "video")],
    );

    let output = run(
        dir.path(),
        &["compose", path(&previous), path(&current), SITE_URL],
        &[],
    );

    assert!(output.status.success(), "{}", log(&output));
    let stdout = stdout(&output);
    assert_eq!(
        stdout.lines().count(),
        1,
        "a step output of GitHub has to be on one line"
    );
    let posts: Vec<String> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(posts.len(), 1);
    assert!(posts[0].contains("\n🎥 Vídeo c\n"), "{}", posts[0]);
    assert!(
        posts[0].ends_with(&format!("🌐 Gráficos: {SITE_URL}")),
        "{}",
        posts[0]
    );
    assert!(log(&output).contains("Vídeo c"), "{}", log(&output));
}

#[test]
fn compose_writes_an_empty_list_when_there_is_no_new_video() {
    let dir = tempfile::tempdir().unwrap();
    let data = data_file(dir.path(), "data.json", &[("a", 2, "video")]);

    let output = run(
        dir.path(),
        &["compose", path(&data), path(&data), SITE_URL],
        &[],
    );

    assert!(output.status.success(), "{}", log(&output));
    assert_eq!(stdout(&output), "[]\n");
}

#[test]
fn compose_fails_when_it_cannot_read_a_data_file() {
    let dir = tempfile::tempdir().unwrap();
    let data = data_file(dir.path(), "data.json", &[("a", 2, "video")]);
    let missing = dir.path().join("missing.json");

    let output = run(
        dir.path(),
        &["compose", path(&missing), path(&data), SITE_URL],
        &[],
    );

    assert!(!output.status.success());
    assert!(log(&output).contains("missing.json"), "{}", log(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn rejects_a_wrong_call_with_its_usage() {
    let dir = tempfile::tempdir().unwrap();

    for args in [&[][..], &["compose", "a", "b"][..], &["tweet"][..]] {
        let output = run(dir.path(), args, &[]);

        assert!(!output.status.success(), "{args:?}");
        assert!(
            log(&output).contains("usage: announce compose PREVIOUS CURRENT SITE_URL"),
            "{}",
            log(&output)
        );
    }
}

#[test]
fn post_publishes_every_text_in_order_signed_with_the_keys() {
    let dir = tempfile::tempdir().unwrap();
    let mut x = Server::new();
    let first = takes(&mut x, "primeiro", "1");
    let second = takes(&mut x, "segundo\n🚨", "2");

    let output = post(dir.path(), &x, &["primeiro", "segundo\n🚨"], true);

    assert!(output.status.success(), "{}", log(&output));
    first.assert();
    second.assert();
    let log = log(&output);
    let first_at = log
        .find("Posted https://x.com/i/status/1:\n    primeiro")
        .expect(&log);
    let second_at = log
        .find("Posted https://x.com/i/status/2:\n    segundo")
        .expect(&log);
    assert!(first_at < second_at, "{log}");
}

#[test]
fn post_stops_at_the_first_text_x_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let mut x = Server::new();
    x.mock("POST", "/2/tweets")
        .match_body(Matcher::Json(json!({"text": "primeiro"})))
        .with_status(403)
        .with_body(json!({"title": "Forbidden", "detail": "Nope.", "status": 403}).to_string())
        .create();
    let second = x
        .mock("POST", "/2/tweets")
        .match_body(Matcher::Json(json!({"text": "segundo"})))
        .expect(0)
        .create();

    let output = post(dir.path(), &x, &["primeiro", "segundo"], true);

    assert!(!output.status.success());
    assert!(
        log(&output).contains(
            "could not post 1 of 2: X post failed with status 403 Forbidden: Forbidden: Nope."
        ),
        "{}",
        log(&output)
    );
    second.assert();
}

#[test]
fn post_only_shows_the_texts_when_the_keys_of_x_are_not_set() {
    let dir = tempfile::tempdir().unwrap();
    let mut x = Server::new();
    let nothing = x.mock("POST", Matcher::Any).expect(0).create();

    let output = post(dir.path(), &x, &["primeiro"], false);

    assert!(output.status.success(), "{}", log(&output));
    assert!(
        log(&output).contains("not posting:\n    primeiro"),
        "{}",
        log(&output)
    );
    nothing.assert();
}

#[test]
fn post_refuses_to_publish_more_texts_at_once_than_a_run_should_ever_find() {
    let dir = tempfile::tempdir().unwrap();
    let mut x = Server::new();
    let nothing = x.mock("POST", Matcher::Any).expect(0).create();

    let output = post(dir.path(), &x, &["1", "2", "3", "4"], true);

    assert!(!output.status.success());
    assert!(
        log(&output).contains("refusing to post 4 texts"),
        "{}",
        log(&output)
    );
    nothing.assert();
}

#[test]
fn post_has_nothing_to_do_without_texts() {
    let dir = tempfile::tempdir().unwrap();
    let mut x = Server::new();
    let nothing = x.mock("POST", Matcher::Any).expect(0).create();

    let output = post(dir.path(), &x, &[], true);

    assert!(output.status.success(), "{}", log(&output));
    assert!(log(&output).contains("Nothing to post"), "{}", log(&output));
    nothing.assert();
}
