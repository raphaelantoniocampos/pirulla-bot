# Pirulla Bot

This project tracks the average length of the videos on the "Pirulla" YouTube channel, publishes it as a website, with the current value of "1 Pirulla", how it changed over time and how each video moved it, and announces every new video on X.

It is all written in Rust, in three parts:

- A generator, the `pirulla-bot` binary, that reads every video of the channel from the YouTube Data API, tells the lives and the shorts from the other videos and writes `site/data.json`.
- A website, the `site/` crate, built with [Leptos](https://leptos.dev) and compiled to WebAssembly, that loads that file, computes the average and draws the interactive charts. Its "Gerar tweet" button writes the post announcing the latest video, to be copied or opened in X and posted by hand.
- An announcer, the `announce` binary, that finds the videos a new data file has and the previous one did not, and posts about them on X.

What they share, the average, how numbers are written in Portuguese and the text of the post, is in the `core/` crate, so the post the bot writes is the same as the one of the button.

The generator keeps no state. Every run rebuilds the whole file from what the channel has at that moment, and when a run fails the previous file is left untouched.

## 🚀 Prerequisites

- [Rust](https://rustup.rs) (stable, 1.88 or newer) and a C compiler, used to build the TLS library
- For the website, the WebAssembly target of Rust (`rustup target add wasm32-unknown-unknown`) and [trunk](https://trunkrs.dev) 0.21
- A [YouTube Data API](https://console.cloud.google.com/apis/library/youtube.googleapis.com) key
- To post on X, the keys of an X app with read and write access, and credits on its account

## ⚙️ Configuration

Create a `.env` file in the directory you run the generator from:

```
DEVELOPER_KEY="Your Google API Developer Key"
CHANNEL_ID="The Channel ID of the channel you want to analyze"
```

To post on X from your machine, add the keys of the app and of the account it posts as. Without them the announcer only shows what it would post.

```
TWITTER_API_KEY="The API key of the app"
TWITTER_API_SECRET="The API key secret of the app"
ACCESS_TOKEN="The access token of the account"
ACCESS_SECRET_TOKEN="The access token secret of the account"
```

## 📦 Usage

Write the data of the website:

```
cargo run --release
```

It writes to `site/data.json`; pass another path as an argument to write somewhere else. Then build the website and serve it:

```
cd site
trunk serve
```

and open http://localhost:8080. `trunk build --release` writes the website to `site/dist`, ready for any static file server.

Find the posts announcing the videos that a data file has and an older one did not:

```
cargo run --bin announce -- compose OLDER.json site/data.json https://example.com/
```

It writes them as a JSON list. `cargo run --bin announce -- post` posts such a list, read from the `POSTS` environment variable, one post after the other.

## 🌐 Publishing

`.github/workflows/site.yml` runs the generator every 30 minutes, builds the website and publishes it to GitHub Pages. Whenever the videos change it also commits `site/data.json`, so the history of the repository keeps every state the channel has been in. To turn it on:

1. In the repository settings, under *Secrets and variables → Actions*, add the secrets `YOUTUBE_DEVELOPER_KEY`, with the key that goes in `DEVELOPER_KEY`, and `CHANNEL_ID`.
2. To post on X, add the four keys of X as secrets too, with the names they have in `.env`, and the variable `X_POSTING` with the value `on`.
3. Under *Pages*, set the source to *GitHub Actions*.
4. Push to `master`.

### Posting on X

Each run compares the data it wrote with the data on `master`, which is what earlier runs have seen. Once the website with the new videos is published, it posts about each new regular video and short, the videos that count in the average the website opens with, in the words of the "Gerar tweet" button. Lives are not announced.

- A video published more than a week before it is found is not announced, and a run that finds more than three new videos posts none and fails: both mean something other than new videos.
- Automated posts must not mention anyone, so an `@name` in a title gets an invisible space after the `@`.
- X charges for each post, more when it has a link, as every announcement does. The account needs credits in the [X developer console](https://console.x.com); without them the post fails.
- When posting fails, the run fails and GitHub sends an email. The videos count as seen by then, so the next run does not try again: re-run the failed job, or post with the "Gerar tweet" button of the website.
- Posting is off unless the repository variable `X_POSTING` is `on`, under *Secrets and variables → Actions → Variables*. While it is off, each run only shows in its log what it would have posted.

GitHub disables scheduled workflows of public repositories after 60 days without activity in the repository, and sends a warning before it does. Re-enable it in the *Actions* tab when that happens.

Anywhere else, schedule `pirulla-bot` with cron, build the website with `trunk build --release` and serve `site/dist`.

## 🧪 Development

```
cargo test
```

It tests the generator, the announcer and `core/`, without the network. The tests replace the YouTube and X APIs with local fake servers, including `tests/cli.rs` and `tests/announce.rs`, which run the compiled binaries from end to end; `YOUTUBE_API_URL` and `X_API_URL` point them at such servers. What the website computes, formatting, grouping by period and the scales of the charts, is tested in `core/`.

The website has tests of its own, which include `site/tests/ui.rs`: it serves the built website with data made for the tests, opens it in Chrome and uses it as a reader would, with the mouse, the keyboard and the switches. Build the website first, then run them:

```
cd site
trunk build --release
cargo test -p pirulla-site
```

Chrome is found on the `PATH`; set `CHROME` to the path of its executable to use another one.

## 🗒️ Notes

Each run costs about 2 units of the YouTube API quota for every 50 videos of the channel, and 1 more for every 50 lives and shorts.

By default the average leaves the lives out and counts the shorts; the page has a switch for each. YouTube has no field that says what a video is, so the generator reads the two playlists YouTube keeps with only the lives and only the shorts of a channel, the same that fill the tabs of the channel page. Their ids are the id of the channel with `UULV` or `UUSH` in the place of its leading `UC`. These playlists are not part of the documented API: if they ever stop existing, every video is taken for a regular one.

`site/data.json` has the time it was generated, the id of the channel and, for every video in order of publication, its `id`, `title`, `publishedAt`, `duration` in seconds and `kind` (`video`, `live` or `short`). It is written with one video per line, so that a new video is a new line in the history of the file.

The website always shows the channel as it is now: a video that is removed from the channel leaves the next file, and the average is computed without it. What the file had before stays in the history of the repository.

## 🖇️ Contributing
If you would like to contribute to this project, please feel free to open an issue or pull request.

## ✒️ Authors
Raphael Campos - [raphaelantoniocampos](https://github.com/raphaelantoniocampos)
