use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, SubsecRound, Utc};
use serde::Serialize;

use crate::series::{Video, timeline};
use crate::youtube::VideoSource;

/// Everything the website shows.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteData {
    pub generated_at: DateTime<Utc>,
    pub channel_id: String,
    pub videos: Vec<Video>,
}

/// Reads the channel `channel_id` from `source` and writes the data file of the website to `output`.
///
/// Nothing is written when the channel cannot be read or has no videos, so a
/// failure never replaces good data.
pub fn generate(
    source: &impl VideoSource,
    channel_id: &str,
    now: DateTime<Utc>,
    output: &Path,
) -> Result<SiteData> {
    let videos = timeline(&source.fetch_videos()?);
    if videos.is_empty() {
        bail!("the channel returned no videos");
    }
    let data = SiteData {
        generated_at: now.trunc_subsecs(0),
        channel_id: channel_id.to_string(),
        videos,
    };
    write(&data, output).with_context(|| format!("could not write {}", output.display()))?;
    Ok(data)
}

/// Writes to a temporary file first, so the website never reads a half-written file.
fn write(data: &SiteData, output: &Path) -> Result<()> {
    if let Some(dir) = output.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut temporary = output.as_os_str().to_owned();
    temporary.push(".tmp");
    let mut writer = BufWriter::new(fs::File::create(&temporary)?);
    write_json(data, &mut writer)?;
    writer.flush()?;
    drop(writer);
    fs::rename(&temporary, output)?;
    Ok(())
}

/// Writes the data as JSON with one video per line: the file is kept under
/// version control, and this way a new video shows up there as a new line.
fn write_json(data: &SiteData, writer: &mut impl Write) -> Result<()> {
    writeln!(writer, "{{")?;
    writeln!(
        writer,
        "\"generatedAt\":{},",
        serde_json::to_string(&data.generated_at)?
    )?;
    writeln!(
        writer,
        "\"channelId\":{},",
        serde_json::to_string(&data.channel_id)?
    )?;
    writeln!(writer, "\"videos\":[")?;
    for (index, video) in data.videos.iter().enumerate() {
        let separator = if index + 1 < data.videos.len() {
            ","
        } else {
            ""
        };
        writeln!(writer, "{}{separator}", serde_json::to_string(video)?)?;
    }
    writeln!(writer, "]")?;
    writeln!(writer, "}}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::series::{Video, VideoKind};
    use anyhow::anyhow;
    use chrono::TimeZone;
    use std::fs;

    struct FakeSource(Option<Vec<Video>>);

    impl VideoSource for FakeSource {
        fn fetch_videos(&self) -> Result<Vec<Video>> {
            self.0.clone().ok_or_else(|| anyhow!("youtube is down"))
        }
    }

    fn day(n: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, n, 12, 0, 0).unwrap()
    }

    fn video(id: &str, title: &str, day_of_month: u32, duration: u64) -> Video {
        Video {
            id: id.to_string(),
            title: title.to_string(),
            published_at: day(day_of_month),
            duration,
            kind: VideoKind::Video,
        }
    }

    fn channel() -> FakeSource {
        FakeSource(Some(vec![
            video("b", "Vídeo \"b\"", 2, 200),
            video("a", "Vídeo a", 1, 100),
        ]))
    }

    #[test]
    fn writes_the_videos_in_order_of_publication_one_per_line() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("data.json");

        generate(&channel(), "UCchannel", day(10), &output).unwrap();

        assert_eq!(
            fs::read_to_string(output).unwrap(),
            r#"{
"generatedAt":"2024-01-10T12:00:00Z",
"channelId":"UCchannel",
"videos":[
{"id":"a","title":"Vídeo a","publishedAt":"2024-01-01T12:00:00Z","duration":100,"kind":"video"},
{"id":"b","title":"Vídeo \"b\"","publishedAt":"2024-01-02T12:00:00Z","duration":200,"kind":"video"}
]
}
"#
        );
    }

    #[test]
    fn the_file_holds_exactly_the_data() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("data.json");

        let data = generate(&channel(), "UC\"channel\"", day(10), &output).unwrap();

        let written: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
        assert_eq!(written, serde_json::to_value(&data).unwrap());
    }

    #[test]
    fn returns_what_was_written() {
        let dir = tempfile::tempdir().unwrap();

        let data = generate(
            &channel(),
            "UCchannel",
            day(10),
            &dir.path().join("data.json"),
        )
        .unwrap();

        assert_eq!(data.generated_at, day(10));
        assert_eq!(data.channel_id, "UCchannel");
        assert_eq!(data.videos.len(), 2);
        assert_eq!(data.videos[1].id, "b");
    }

    #[test]
    fn the_time_of_generation_has_no_fraction_of_a_second() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("data.json");

        generate(
            &channel(),
            "UCchannel",
            day(10) + chrono::Duration::nanoseconds(123_456_789),
            &output,
        )
        .unwrap();

        assert!(
            fs::read_to_string(output)
                .unwrap()
                .contains(r#""generatedAt":"2024-01-10T12:00:00Z""#)
        );
    }

    #[test]
    fn creates_the_directory_of_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("site").join("data.json");

        generate(&channel(), "UCchannel", day(10), &output).unwrap();

        assert!(output.exists());
    }

    #[test]
    fn replaces_the_previous_file_and_leaves_no_other_file_behind() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("data.json");
        fs::write(&output, "old").unwrap();

        generate(&channel(), "UCchannel", day(10), &output).unwrap();

        assert!(fs::read_to_string(&output).unwrap().starts_with('{'));
        let files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(files, ["data.json"]);
    }

    #[test]
    fn a_failure_to_read_the_channel_keeps_the_previous_file() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("data.json");
        fs::write(&output, "old").unwrap();

        let error = generate(&FakeSource(None), "UCchannel", day(10), &output).unwrap_err();

        assert!(format!("{error:#}").contains("youtube is down"));
        assert_eq!(fs::read_to_string(output).unwrap(), "old");
    }

    #[test]
    fn a_channel_without_videos_keeps_the_previous_file() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("data.json");
        fs::write(&output, "old").unwrap();
        let only_a_live = FakeSource(Some(vec![video("live", "Ao vivo", 1, 0)]));

        assert!(generate(&FakeSource(Some(vec![])), "UCchannel", day(10), &output).is_err());
        assert!(generate(&only_a_live, "UCchannel", day(10), &output).is_err());
        assert_eq!(fs::read_to_string(output).unwrap(), "old");
    }
}
