use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::duration::parse_iso8601_duration;
use crate::http;
use crate::series::Video;

const API_URL: &str = "https://www.googleapis.com/youtube/v3";
/// The most items the API returns or accepts per request.
const PAGE_SIZE: usize = 50;

/// Where the videos of the channel come from.
pub trait VideoSource {
    fn fetch_videos(&self) -> Result<Vec<Video>>;
}

/// Lists the uploads of a channel through the YouTube Data API.
pub struct YoutubeClient {
    http: Client,
    base_url: String,
    api_key: String,
    channel_id: String,
}

impl YoutubeClient {
    pub fn new(api_key: impl Into<String>, channel_id: impl Into<String>) -> Result<Self> {
        Ok(Self {
            http: http::client()?,
            base_url: API_URL.to_string(),
            api_key: api_key.into(),
            channel_id: channel_id.into(),
        })
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn get<T: DeserializeOwned>(&self, resource: &str, params: &[(&str, &str)]) -> Result<T> {
        let request = self
            .http
            .get(format!("{}/{resource}", self.base_url))
            .query(params)
            .query(&[("key", &self.api_key)]);
        http::send(request, &format!("YouTube {resource} request"))
    }

    fn uploads_playlist_id(&self) -> Result<String> {
        let channels: ChannelList = self.get(
            "channels",
            &[("part", "contentDetails"), ("id", &self.channel_id)],
        )?;
        match channels.items.into_iter().next() {
            Some(channel) => Ok(channel.content_details.related_playlists.uploads),
            None => bail!("YouTube channel {} was not found", self.channel_id),
        }
    }

    fn playlist_video_ids(&self, playlist_id: &str) -> Result<Vec<String>> {
        let page_size = PAGE_SIZE.to_string();
        let mut ids = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut params = vec![
                ("part", "contentDetails"),
                ("playlistId", playlist_id),
                ("maxResults", page_size.as_str()),
            ];
            if let Some(token) = &page_token {
                params.push(("pageToken", token));
            }
            let page: PlaylistPage = self.get("playlistItems", &params)?;
            ids.extend(
                page.items
                    .into_iter()
                    .map(|item| item.content_details.video_id),
            );
            page_token = page.next_page_token;
            if page_token.is_none() {
                return Ok(ids);
            }
        }
    }

    fn video_details(&self, ids: &[String]) -> Result<Vec<Video>> {
        let mut videos = Vec::with_capacity(ids.len());
        for batch in ids.chunks(PAGE_SIZE) {
            let list: VideoList = self.get(
                "videos",
                &[("part", "snippet,contentDetails"), ("id", &batch.join(","))],
            )?;
            videos.extend(list.items.into_iter().map(|item| Video {
                id: item.id,
                title: item.snippet.title,
                published_at: item.snippet.published_at,
                duration: parse_iso8601_duration(&item.content_details.duration).unwrap_or(0),
            }));
        }
        Ok(videos)
    }
}

impl VideoSource for YoutubeClient {
    fn fetch_videos(&self) -> Result<Vec<Video>> {
        let playlist_id = self.uploads_playlist_id()?;
        let ids = self.playlist_video_ids(&playlist_id)?;
        self.video_details(&ids)
    }
}

#[derive(Deserialize)]
struct ChannelList {
    #[serde(default)]
    items: Vec<Channel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Channel {
    content_details: ChannelDetails,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChannelDetails {
    related_playlists: RelatedPlaylists,
}

#[derive(Deserialize)]
struct RelatedPlaylists {
    uploads: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistPage {
    #[serde(default)]
    items: Vec<PlaylistItem>,
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistItem {
    content_details: PlaylistItemDetails,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistItemDetails {
    video_id: String,
}

#[derive(Deserialize)]
struct VideoList {
    #[serde(default)]
    items: Vec<VideoItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoItem {
    id: String,
    snippet: Snippet,
    content_details: VideoDetails,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Snippet {
    title: String,
    published_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct VideoDetails {
    #[serde(default)]
    duration: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use mockito::{Matcher, Mock, Server, ServerGuard};
    use serde_json::{Value, json};

    const API_KEY: &str = "secret-api-key";

    fn client(server: &ServerGuard) -> YoutubeClient {
        YoutubeClient::new(API_KEY, "UCchannel")
            .unwrap()
            .with_base_url(server.url())
    }

    fn query(params: &[(&str, &str)]) -> Matcher {
        Matcher::AllOf(
            params
                .iter()
                .map(|(name, value)| Matcher::UrlEncoded(name.to_string(), value.to_string()))
                .collect(),
        )
    }

    fn mock_channel(server: &mut ServerGuard) -> Mock {
        server
            .mock("GET", "/channels")
            .match_query(query(&[("part", "contentDetails"), ("id", "UCchannel"), ("key", API_KEY)]))
            .with_body(json!({"items": [{"contentDetails": {"relatedPlaylists": {"uploads": "UUchannel"}}}]}).to_string())
            .expect(1)
            .create()
    }

    fn mock_playlist_page(
        server: &mut ServerGuard,
        params: &[(&str, &str)],
        ids: &[String],
        next: Option<&str>,
    ) -> Mock {
        let items: Vec<Value> = ids
            .iter()
            .map(|id| json!({"contentDetails": {"videoId": id}}))
            .collect();
        let mut body = json!({"items": items});
        if let Some(token) = next {
            body["nextPageToken"] = json!(token);
        }
        let mut params = params.to_vec();
        params.extend([
            ("part", "contentDetails"),
            ("playlistId", "UUchannel"),
            ("maxResults", "50"),
            ("key", API_KEY),
        ]);
        server
            .mock("GET", "/playlistItems")
            .match_query(query(&params))
            .with_body(body.to_string())
            .expect(1)
            .create()
    }

    fn mock_videos(server: &mut ServerGuard, ids: &[String], items: Value) -> Mock {
        server
            .mock("GET", "/videos")
            .match_query(query(&[
                ("part", "snippet,contentDetails"),
                ("id", &ids.join(",")),
                ("key", API_KEY),
            ]))
            .with_body(json!({"items": items}).to_string())
            .expect(1)
            .create()
    }

    fn video_json(id: &str, title: &str, published_at: &str, duration: &str) -> Value {
        json!({
            "id": id,
            "snippet": {"title": title, "publishedAt": published_at},
            "contentDetails": {"duration": duration},
        })
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn lists_the_videos_of_every_page_of_the_uploads_playlist() {
        let mut server = Server::new();
        let channel = mock_channel(&mut server);
        // Created before the first page so that it wins when both match.
        let second_page = mock_playlist_page(
            &mut server,
            &[("pageToken", "PAGE2")],
            &strings(&["v1"]),
            None,
        );
        let first_page =
            mock_playlist_page(&mut server, &[], &strings(&["v3", "v2"]), Some("PAGE2"));
        let details = mock_videos(
            &mut server,
            &strings(&["v3", "v2", "v1"]),
            json!([
                video_json("v3", "Terceiro", "2024-01-03T12:00:00Z", "PT1H2M3S"),
                video_json("v2", "Segundo", "2024-01-02T12:00:00Z", "PT10M"),
                video_json("v1", "Primeiro", "2024-01-01T12:00:00Z", "PT45S"),
            ]),
        );

        let videos = client(&server).fetch_videos().unwrap();

        assert_eq!(
            videos,
            [
                Video {
                    id: "v3".to_string(),
                    title: "Terceiro".to_string(),
                    published_at: Utc.with_ymd_and_hms(2024, 1, 3, 12, 0, 0).unwrap(),
                    duration: 3723,
                },
                Video {
                    id: "v2".to_string(),
                    title: "Segundo".to_string(),
                    published_at: Utc.with_ymd_and_hms(2024, 1, 2, 12, 0, 0).unwrap(),
                    duration: 600,
                },
                Video {
                    id: "v1".to_string(),
                    title: "Primeiro".to_string(),
                    published_at: Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap(),
                    duration: 45,
                },
            ]
        );
        for mock in [channel, first_page, second_page, details] {
            mock.assert();
        }
    }

    #[test]
    fn asks_for_the_details_of_at_most_fifty_videos_at_a_time() {
        let mut server = Server::new();
        let ids: Vec<String> = (1..=51).map(|n| format!("v{n}")).collect();
        mock_channel(&mut server);
        mock_playlist_page(&mut server, &[], &ids, None);
        let details = |ids: &[String]| -> Value {
            ids.iter()
                .map(|id| video_json(id, id, "2024-01-01T12:00:00Z", "PT1M"))
                .collect()
        };
        let first_batch = mock_videos(&mut server, &ids[..50], details(&ids[..50]));
        let second_batch = mock_videos(&mut server, &ids[50..], details(&ids[50..]));

        let videos = client(&server).fetch_videos().unwrap();

        assert_eq!(videos.len(), 51);
        first_batch.assert();
        second_batch.assert();
    }

    #[test]
    fn videos_whose_duration_is_unknown_have_no_duration() {
        let mut server = Server::new();
        mock_channel(&mut server);
        mock_playlist_page(&mut server, &[], &strings(&["live", "odd"]), None);
        mock_videos(
            &mut server,
            &strings(&["live", "odd"]),
            json!([
                video_json("live", "Ao vivo", "2024-01-02T12:00:00Z", "P0D"),
                video_json("odd", "Estranho", "2024-01-01T12:00:00Z", "soon"),
            ]),
        );

        let videos = client(&server).fetch_videos().unwrap();

        assert_eq!(
            videos.iter().map(|v| v.duration).collect::<Vec<_>>(),
            [0, 0]
        );
    }

    #[test]
    fn fails_when_the_channel_does_not_exist() {
        let mut server = Server::new();
        server
            .mock("GET", "/channels")
            .match_query(Matcher::Any)
            .with_body(json!({"pageInfo": {"totalResults": 0}}).to_string())
            .create();

        let error = client(&server).fetch_videos().unwrap_err();

        assert!(format!("{error:#}").contains("UCchannel"), "{error:#}");
    }

    #[test]
    fn reports_the_reason_given_by_the_api_without_leaking_the_key() {
        let mut server = Server::new();
        server
            .mock("GET", "/channels")
            .match_query(Matcher::Any)
            .with_status(403)
            .with_body(
                json!({"error": {"code": 403, "message": "You have exceeded your quota."}})
                    .to_string(),
            )
            .create();

        let error = format!("{:#}", client(&server).fetch_videos().unwrap_err());

        assert!(error.contains("403"), "{error}");
        assert!(error.contains("You have exceeded your quota."), "{error}");
        assert!(!error.contains(API_KEY), "{error}");
    }

    #[test]
    fn a_connection_failure_does_not_leak_the_key() {
        let unreachable = YoutubeClient::new(API_KEY, "UCchannel")
            .unwrap()
            .with_base_url("http://127.0.0.1:1");

        let error = format!("{:#}", unreachable.fetch_videos().unwrap_err());

        assert!(!error.contains(API_KEY), "{error}");
    }
}
