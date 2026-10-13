use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, bail};
use chrono::Utc;
use log::{error, info};

use pirulla_bot::config::Config;
use pirulla_bot::duration::format_time;
use pirulla_bot::site::generate;
use pirulla_bot::youtube::YoutubeClient;

const USAGE: &str = "usage: pirulla-bot [OUTPUT]\n\n  OUTPUT  where to write the data of the website (default: site/data.json)";
const DEFAULT_OUTPUT: &str = "site/data.json";

fn main() -> ExitCode {
    dotenvy::dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let output = match args.as_slice() {
        [] => PathBuf::from(DEFAULT_OUTPUT),
        [path] if !path.starts_with('-') => PathBuf::from(path),
        _ => bail!("{USAGE}"),
    };
    let config = Config::from_env()?;
    let mut youtube = YoutubeClient::new(&config.developer_key, &config.channel_id)?;
    if let Some(url) = &config.youtube_api_url {
        youtube = youtube.with_base_url(url);
    }

    let data = generate(&youtube, &config.channel_id, Utc::now(), &output)?;
    let current_mean = data.videos.last().map_or(0.0, |video| video.mean);
    info!(
        "Wrote {} videos to {}: 1 Pirulla = {}",
        data.videos.len(),
        output.display(),
        format_time(current_mean)
    );
    Ok(())
}
