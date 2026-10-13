# Pirulla Bot

This project tracks the average length of the videos on the "Pirulla" YouTube channel and publishes it as a website: the current value of "1 Pirulla", how it changed over time and how each video moved it.

It has two parts:

- A generator, written in Rust, that reads every video of the channel from the YouTube Data API, computes the running average and writes `site/data.json`.
- A static website in `site/`, in plain HTML, CSS and JavaScript with no dependencies, that loads that file and draws the interactive charts.

The generator keeps no state. Every run rebuilds the whole file from what the channel has at that moment, and when a run fails the previous file is left untouched.

## 🚀 Prerequisites

- [Rust](https://rustup.rs) (stable; developed with 1.93) and a C compiler, used to build the TLS library
- A [YouTube Data API](https://console.cloud.google.com/apis/library/youtube.googleapis.com) key
- [Node.js](https://nodejs.org) 22 or newer, only to run the tests of the website

## ⚙️ Configuration

Create a `.env` file in the directory you run the generator from:

```
DEVELOPER_KEY="Your Google API Developer Key"
CHANNEL_ID="The Channel ID of the channel you want to analyze"
```

## 📦 Usage

Write the data of the website:

```
cargo run --release
```

It writes to `site/data.json`; pass another path as an argument to write somewhere else. Then serve the `site` directory with any static file server, for example:

```
python3 -m http.server --directory site
```

and open http://localhost:8000. The page has to be served: opened straight from the disk, the browser does not let it load the data file.

## 🌐 Publishing

`.github/workflows/site.yml` runs the generator every 30 minutes and publishes `site/` to GitHub Pages. Whenever the videos change it also commits `site/data.json`, so the history of the repository keeps every state the channel has been in. To turn it on:

1. In the repository settings, under *Secrets and variables → Actions*, add the secrets `YOUTUBE_DEVELOPER_KEY`, with the key that goes in `DEVELOPER_KEY`, and `CHANNEL_ID`.
2. Under *Pages*, set the source to *GitHub Actions*.
3. Push to `master`.

GitHub disables scheduled workflows of public repositories after 60 days without activity in the repository, and sends a warning before it does. Re-enable it in the *Actions* tab when that happens.

Anywhere else, schedule `pirulla-bot` with cron and serve the `site` directory.

## 🧪 Development

```
cargo test
npm test
```

Neither uses the network. The Rust tests replace the YouTube API with a local fake server, including `tests/cli.rs`, which runs the compiled generator from end to end; `YOUTUBE_API_URL` points the generator at such a server. `npm test` covers the functions of the website that do not touch the page: formatting, grouping by period and the scales of the charts.

## 🗒️ Notes

Each run costs about 2 units of the YouTube API quota for every 50 videos of the channel.

`site/data.json` has the time it was generated, the id of the channel and, for every video in order of publication, its `id`, `title`, `publishedAt`, `duration` in seconds and `mean`, the average duration in seconds of all videos published up to it. It is written with one video per line, so that a new video is a new line in the history of the file.

The website always shows the channel as it is now: a video that is removed from the channel leaves the next file, and the averages after it change. What the file had before stays in the history of the repository.

## 🖇️ Contributing
If you would like to contribute to this project, please feel free to open an issue or pull request.

## ✒️ Authors
Raphael Campos - [raphaelantoniocampos](https://github.com/raphaelantoniocampos)
