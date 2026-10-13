//! Runs the compiled generator against a fake YouTube server.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use mockito::{Matcher, Server, ServerGuard};
use serde_json::{Value, json};

/// A fake YouTube whose channel has three videos, listed newest first.
fn youtube() -> ServerGuard {
    let mut youtube = Server::new();
    let videos = [
        ("v3", "Terceiro", "2024-03-01T12:00:00Z", "PT1H"),
        ("v2", "Segundo", "2024-02-01T12:00:00Z", "PT30M"),
        ("v1", "Primeiro", "2024-01-01T12:00:00Z", "PT20M"),
    ];
    let details: Vec<Value> = videos
        .iter()
        .map(|v| json!({"id": v.0, "snippet": {"title": v.1, "publishedAt": v.2}, "contentDetails": {"duration": v.3}}))
        .collect();
    youtube
        .mock("GET", "/channels")
        .match_query(Matcher::Any)
        .with_body(
            json!({"items": [{"contentDetails": {"relatedPlaylists": {"uploads": "UUchannel"}}}]})
                .to_string(),
        )
        .create();
    let playlist = |ids: &[&str]| {
        let items: Vec<Value> = ids
            .iter()
            .map(|id| json!({"contentDetails": {"videoId": id}}))
            .collect();
        json!({"items": items}).to_string()
    };
    // The uploads of the channel, and the playlists YouTube keeps with only its lives and only its shorts.
    for (playlist_id, ids) in [
        ("UUchannel", &["v3", "v2", "v1"][..]),
        ("UULVchannel", &["v3"][..]),
        ("UUSHchannel", &["v1"][..]),
    ] {
        youtube
            .mock("GET", "/playlistItems")
            .match_query(Matcher::UrlEncoded(
                "playlistId".to_string(),
                playlist_id.to_string(),
            ))
            .with_body(playlist(ids))
            .create();
    }
    youtube
        .mock("GET", "/videos")
        .match_query(Matcher::Any)
        .with_body(json!({"items": details}).to_string())
        .create();
    youtube
}

/// Runs the generator in `dir`, isolated from the environment of whoever runs the tests.
fn run(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pirulla-bot"))
        .args(args)
        .current_dir(dir)
        .env_clear()
        .envs(env.iter().copied())
        .output()
        .unwrap()
}

fn log(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn settings(youtube_url: &str) -> Vec<(&str, &str)> {
    vec![
        ("DEVELOPER_KEY", "developer-key"),
        ("CHANNEL_ID", "UCchannel"),
        ("YOUTUBE_API_URL", youtube_url),
    ]
}

#[test]
fn writes_the_data_of_the_site() {
    let dir = tempfile::tempdir().unwrap();
    let youtube = youtube();

    let output = run(dir.path(), &[], &settings(&youtube.url()));

    assert!(output.status.success(), "{}", log(&output));
    let data: Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join("site/data.json")).unwrap())
            .unwrap();
    assert!(data["generatedAt"].is_string());
    assert_eq!(data["channelId"], "UCchannel");
    assert_eq!(
        data["videos"],
        json!([
            {"id": "v1", "title": "Primeiro", "publishedAt": "2024-01-01T12:00:00Z", "duration": 1200, "kind": "short"},
            {"id": "v2", "title": "Segundo", "publishedAt": "2024-02-01T12:00:00Z", "duration": 1800, "kind": "video"},
            {"id": "v3", "title": "Terceiro", "publishedAt": "2024-03-01T12:00:00Z", "duration": 3600, "kind": "live"},
        ])
    );
    assert!(
        log(&output).contains("Wrote 3 videos to site/data.json (regular: 1, lives: 1, shorts: 1)"),
        "{}",
        log(&output)
    );
}

#[test]
fn writes_to_the_path_given_as_argument() {
    let dir = tempfile::tempdir().unwrap();
    let youtube = youtube();

    let output = run(
        dir.path(),
        &["public/pirulla.json"],
        &settings(&youtube.url()),
    );

    assert!(output.status.success(), "{}", log(&output));
    assert!(dir.path().join("public/pirulla.json").exists());
    assert!(!dir.path().join("site").exists());
}

#[test]
fn a_failed_run_is_reported_and_keeps_the_previous_data() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("data.json"), "previous data").unwrap();
    let mut youtube = Server::new();
    youtube
        .mock("GET", "/channels")
        .match_query(Matcher::Any)
        .with_status(403)
        .with_body(json!({"error": {"code": 403, "message": "quotaExceeded"}}).to_string())
        .create();

    let output = run(dir.path(), &["data.json"], &settings(&youtube.url()));

    assert!(!output.status.success());
    assert!(log(&output).contains("quotaExceeded"), "{}", log(&output));
    assert!(!log(&output).contains("developer-key"), "{}", log(&output));
    assert_eq!(
        fs::read_to_string(dir.path().join("data.json")).unwrap(),
        "previous data"
    );
}

#[test]
fn refuses_to_start_without_its_settings() {
    let dir = tempfile::tempdir().unwrap();

    let output = run(dir.path(), &[], &[("CHANNEL_ID", "UCchannel")]);

    assert!(!output.status.success());
    assert!(log(&output).contains("DEVELOPER_KEY"), "{}", log(&output));
    assert!(!dir.path().join("site").exists());
}

#[test]
fn rejects_arguments_it_does_not_know() {
    let dir = tempfile::tempdir().unwrap();

    for args in [&["--once"][..], &["a.json", "b.json"][..]] {
        let output = run(dir.path(), args, &[]);

        assert!(!output.status.success(), "{args:?}");
        assert!(
            log(&output).contains("usage: pirulla-bot [OUTPUT]"),
            "{}",
            log(&output)
        );
    }
}
