use std::path::Path;
use std::process::ExitCode;

use anyhow::{Result, bail};
use chrono::Utc;
use log::{error, info};

use pirulla_bot::announce::{DryRun, announce, compose};
use pirulla_bot::config::PostConfig;
use pirulla_bot::x::XClient;

const USAGE: &str = "usage: announce compose PREVIOUS CURRENT SITE_URL
       announce post

  compose  writes, as a JSON list on one line, the posts announcing the videos
           that the data file CURRENT has and the data file PREVIOUS did not
  post     posts on X, one after the other, the texts of such a list, read from
           the POSTS environment variable; without the keys of X it only shows them";

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
    match args.as_slice() {
        [command, previous, current, site_url] if command == "compose" => {
            let posts = compose(
                Path::new(previous),
                Path::new(current),
                site_url,
                Utc::now(),
            )?;
            if posts.is_empty() {
                info!("No new video to announce");
            }
            for post in &posts {
                info!("To announce:\n{post}");
            }
            println!("{}", serde_json::to_string(&posts)?);
            Ok(())
        }
        [command] if command == "post" => post(PostConfig::from_env()?),
        _ => bail!("{USAGE}"),
    }
}

fn post(config: PostConfig) -> Result<()> {
    if config.posts.is_empty() {
        info!("Nothing to post");
        return Ok(());
    }
    match config.x_credentials {
        Some(credentials) => {
            let mut x = XClient::new(credentials)?;
            if let Some(url) = &config.x_api_url {
                x = x.with_base_url(url);
            }
            announce(&config.posts, &x)
        }
        None => announce(&config.posts, &DryRun),
    }
}
